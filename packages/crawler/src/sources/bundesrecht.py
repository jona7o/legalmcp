"""Bundesrecht crawler for Gesetze-im-Internet."""

import logging
from typing import List, Dict, Any
from datetime import datetime
from urllib.parse import urljoin

from .base import BaseCrawler, CrawlStats
from ..db import SourceType, DocumentType, DocumentOperations, db
from ..processing import XMLParser, TextNormalizer, TextChunker, get_embedder
from ..config import settings

logger = logging.getLogger(__name__)


class BundesrechtCrawler(BaseCrawler):
    """Crawler for German federal law (Gesetze-im-Internet)."""
    
    @property
    def source_type(self) -> SourceType:
        return SourceType.BUNDESRECHT
    
    @property
    def name(self) -> str:
        return "Bundesrecht"
    
    def __init__(self):
        super().__init__()
        self.base_url = "https://www.gesetze-im-internet.de"
        self.toc_url = settings.bundesrecht_xml_url
        self.parser = XMLParser()
        self.normalizer = TextNormalizer()
        self.chunker = TextChunker(
            max_tokens=settings.chunk_size,
            overlap_tokens=settings.chunk_overlap
        )
    
    async def crawl(self) -> CrawlStats:
        """Crawl Bundesrecht documents."""
        stats = CrawlStats()
        
        logger.info(f"Starting {self.name} crawl from {self.toc_url}")
        
        # Check robots.txt
        if not await self.check_robots_txt(self.base_url):
            logger.warning(f"Robots.txt check failed for {self.base_url}")
            # Continue anyway for official government sources
        
        try:
            # Fetch table of contents
            toc_xml = await self.fetch_text(self.toc_url)
            documents = self.parser.parse_bundesrecht_toc(toc_xml)
            stats.documents_found = len(documents)
            
            logger.info(f"Found {len(documents)} documents in TOC")
            
            # Process each document
            for i, doc_info in enumerate(documents):
                try:
                    await self._process_document(doc_info, stats)
                    
                    if (i + 1) % 10 == 0:
                        self.log_progress(i + 1, len(documents))
                
                except Exception as e:
                    stats.documents_failed += 1
                    error_msg = f"Failed to process {doc_info.get('title', 'unknown')}: {e}"
                    stats.errors.append(error_msg)
                    logger.error(error_msg)
            
            logger.info(f"Bundesrecht crawl completed: {stats}")
        
        except Exception as e:
            logger.error(f"Bundesrecht crawl failed: {e}")
            stats.errors.append(str(e))
            raise
        
        return stats
    
    async def _process_document(self, doc_info: Dict[str, str], stats: CrawlStats):
        """Process a single Bundesrecht document."""
        title = doc_info['title']
        xml_url = doc_info['link']
        
        # Make URL absolute
        if not xml_url.startswith('http'):
            xml_url = urljoin(self.base_url, xml_url)
        
        # Fetch XML
        try:
            xml_content = await self.fetch_text(xml_url, encoding='utf-8')
        except Exception as e:
            logger.warning(f"Failed to fetch {xml_url}: {e}")
            # Try ISO-8859-1 encoding
            xml_content = await self.fetch_text(xml_url, encoding='iso-8859-1')
        
        # Parse XML
        parsed = self.parser.parse_bundesrecht_xml(xml_content)
        full_text = parsed['full_text']
        
        if not full_text:
            logger.warning(f"No text content found for {title}")
            return
        
        # Normalize text
        full_text = self.normalizer.normalize(full_text)
        
        # Generate source ID
        source_id = self._generate_source_id(doc_info)
        
        # Check if document needs update
        async with await db.get_session() as session:
            doc_ops = DocumentOperations(session)
            content_hash = doc_ops.calculate_content_hash(full_text)
            needs_update, existing_doc = await doc_ops.document_needs_update(
                source_id, content_hash
            )
            
            if not needs_update:
                stats.documents_unchanged += 1
                logger.debug(f"Document unchanged: {title}")
                return
            
            # Create or update document
            if existing_doc:
                document = await doc_ops.update_document(
                    existing_doc,
                    full_text=full_text,
                    title=title,
                    abbreviation=doc_info.get('abbreviation', ''),
                    source_url=xml_url.replace('/xml.zip', ''),
                    xml_url=xml_url,
                )
                stats.documents_updated += 1
                logger.info(f"Updated: {title}")
            else:
                document = await doc_ops.create_document(
                    source=SourceType.BUNDESRECHT,
                    source_id=source_id,
                    title=title,
                    document_type=self._determine_document_type(title),
                    full_text=full_text,
                    abbreviation=doc_info.get('abbreviation', ''),
                    jurisdiction='Bund',
                    in_force=True,
                    source_url=xml_url.replace('/xml.zip', ''),
                    xml_url=xml_url,
                )
                stats.documents_new += 1
                logger.info(f"Created: {title}")
            
            # Generate chunks
            chunks_data = self.chunker.chunk_document(
                full_text,
                metadata={'document_title': title}
            )
            
            # Delete old chunks
            await doc_ops.delete_document_chunks(document.id)
            
            # Create new chunks
            chunks = await doc_ops.create_chunks(document.id, chunks_data)
            
            # Generate embeddings
            embedder = get_embedder()
            texts = [chunk.text for chunk in chunks]
            embeddings = embedder.embed_texts(texts)
            
            # Update chunks with embeddings
            for chunk, embedding in zip(chunks, embeddings):
                await doc_ops.update_chunk_embedding(chunk.id, embedding)
            
            await doc_ops.commit()
            
            logger.debug(f"Created {len(chunks)} chunks for {title}")
    
    def _generate_source_id(self, doc_info: Dict[str, str]) -> str:
        """Generate unique source ID for document."""
        # Use abbreviation or extract from link
        abbr = doc_info.get('abbreviation', '')
        if abbr:
            return f"bundesrecht_{abbr.lower()}"
        
        # Extract from URL
        link = doc_info['link']
        if '/' in link:
            law_id = link.split('/')[-2] if link.endswith('/') else link.split('/')[-1]
            law_id = law_id.replace('xml.zip', '').strip('/')
            return f"bundesrecht_{law_id}"
        
        # Fallback to title hash
        import hashlib
        title_hash = hashlib.md5(doc_info['title'].encode()).hexdigest()[:8]
        return f"bundesrecht_{title_hash}"
    
    def _determine_document_type(self, title: str) -> DocumentType:
        """Determine document type from title."""
        title_lower = title.lower()
        
        if 'verordnung' in title_lower:
            return DocumentType.VERORDNUNG
        else:
            return DocumentType.BUNDESGESETZ
