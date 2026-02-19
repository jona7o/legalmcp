"""Bayern.Recht crawler for Bavarian state law."""

import logging
import hashlib
from typing import List, Dict, Any
from datetime import datetime
from urllib.parse import urljoin, urlparse
import asyncio

from .base import BaseCrawler, CrawlStats
from ..db import SourceType, DocumentType, DocumentOperations, db
from ..processing import HTMLParser, TextNormalizer, TextChunker, get_embedder
from ..config import settings

logger = logging.getLogger(__name__)


class BayernCrawler(BaseCrawler):
    """Crawler for Bavarian state law (Bayern.Recht)."""
    
    @property
    def source_type(self) -> SourceType:
        return SourceType.BAYERN
    
    @property
    def name(self) -> str:
        return "Bayern.Recht"
    
    def __init__(self):
        super().__init__()
        self.base_url = settings.bayern_base_url
        self.parser = HTMLParser()
        self.normalizer = TextNormalizer()
        self.chunker = TextChunker(
            max_tokens=settings.chunk_size,
            overlap_tokens=settings.chunk_overlap
        )
        # Specific rate limit for Bayern.Recht (be respectful)
        self.rate_limiter.max_rate = 1.0  # 1 request per second
    
    async def crawl(self) -> CrawlStats:
        """Crawl Bayern.Recht documents."""
        stats = CrawlStats()
        
        logger.info(f"Starting {self.name} crawl from {self.base_url}")
        
        # Check robots.txt
        if not await self.check_robots_txt(self.base_url):
            logger.warning(f"Robots.txt check failed for {self.base_url}")
        
        try:
            # Get list of laws from index page
            law_links = await self._fetch_law_index()
            stats.documents_found = len(law_links)
            
            logger.info(f"Found {len(law_links)} laws in index")
            
            # Process each law
            for i, law_url in enumerate(law_links):
                try:
                    await self._process_law(law_url, stats)
                    
                    if (i + 1) % 5 == 0:
                        self.log_progress(i + 1, len(law_links))
                    
                    # Add delay between requests
                    await asyncio.sleep(1.0)
                
                except Exception as e:
                    stats.documents_failed += 1
                    error_msg = f"Failed to process {law_url}: {e}"
                    stats.errors.append(error_msg)
                    logger.error(error_msg)
            
            logger.info(f"Bayern.Recht crawl completed: {stats}")
        
        except Exception as e:
            logger.error(f"Bayern.Recht crawl failed: {e}")
            stats.errors.append(str(e))
            raise
        
        return stats
    
    async def _fetch_law_index(self) -> List[str]:
        """
        Fetch list of law URLs from index page.
        
        This is a simplified implementation. In production, you would need to:
        1. Navigate the actual Bayern.Recht structure
        2. Handle pagination
        3. Extract proper law URLs
        """
        # For demo purposes, return a sample list
        # In production, scrape the actual index page
        
        try:
            # Main index URL - adjust based on actual site structure
            index_url = f"{self.base_url}/Content/Catalog"
            html = await self.fetch_text(index_url)
            
            # Parse HTML to find law links
            from bs4 import BeautifulSoup
            soup = BeautifulSoup(html, 'lxml')
            
            # Find all law links - adjust selectors based on actual HTML
            links = []
            for link in soup.find_all('a', href=True):
                href = link['href']
                # Filter for actual law pages
                if '/Document/' in href or '/Norm/' in href:
                    full_url = urljoin(self.base_url, href)
                    if full_url not in links:
                        links.append(full_url)
            
            return links[:100]  # Limit for testing
        
        except Exception as e:
            logger.warning(f"Could not fetch law index: {e}")
            # Return empty list or sample URLs for testing
            return []
    
    async def _process_law(self, law_url: str, stats: CrawlStats):
        """Process a single Bayern.Recht law."""
        # Fetch HTML
        html_content = await self.fetch_text(law_url)
        
        # Parse HTML
        parsed = self.parser.parse_bayern_law(html_content)
        
        title = parsed.get('title', '')
        full_text = parsed.get('full_text', '')
        
        if not full_text or not title:
            logger.warning(f"No content found for {law_url}")
            return
        
        # Normalize text
        full_text = self.normalizer.normalize(full_text)
        
        # Generate source ID from URL
        source_id = self._generate_source_id(law_url)
        
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
            
            # Extract metadata
            metadata = {
                'sections': parsed.get('sections', []),
            }
            
            # Create or update document
            if existing_doc:
                document = await doc_ops.update_document(
                    existing_doc,
                    full_text=full_text,
                    title=title,
                    source_url=law_url,
                    metadata=metadata,
                )
                stats.documents_updated += 1
                logger.info(f"Updated: {title}")
            else:
                document = await doc_ops.create_document(
                    source=SourceType.BAYERN,
                    source_id=source_id,
                    title=title,
                    document_type=DocumentType.LANDESGESETZ,
                    full_text=full_text,
                    jurisdiction='Bayern',
                    in_force=True,
                    source_url=law_url,
                    metadata=metadata,
                )
                stats.documents_new += 1
                logger.info(f"Created: {title}")
            
            # Generate chunks with section metadata
            sections = parsed.get('sections', [])
            if sections:
                chunks_data = self.chunker.chunk_with_sections(
                    sections,
                    base_metadata={'document_title': title}
                )
            else:
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
    
    def _generate_source_id(self, url: str) -> str:
        """Generate unique source ID from URL."""
        # Extract path and use as ID
        parsed = urlparse(url)
        path = parsed.path.strip('/')
        
        # Clean up path
        source_id = path.replace('/', '_').replace('.', '_')
        
        # If too long, use hash
        if len(source_id) > 200:
            url_hash = hashlib.md5(url.encode()).hexdigest()[:12]
            source_id = f"bayern_{url_hash}"
        else:
            source_id = f"bayern_{source_id}"
        
        return source_id
