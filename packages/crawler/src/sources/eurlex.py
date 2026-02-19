"""EUR-Lex crawler for European Union legislation."""

import logging
import hashlib
from typing import List, Dict, Any, Optional
from datetime import datetime
from urllib.parse import urljoin
import asyncio

from .base import BaseCrawler, CrawlStats
from ..db import SourceType, DocumentType, DocumentOperations, db
from ..processing import EURLexParser, HTMLParser, TextNormalizer, TextChunker, get_embedder
from ..config import settings

logger = logging.getLogger(__name__)


class EURLexCrawler(BaseCrawler):
    """Crawler for EU legislation (EUR-Lex)."""
    
    @property
    def source_type(self) -> SourceType:
        return SourceType.EURLEX
    
    @property
    def name(self) -> str:
        return "EUR-Lex"
    
    def __init__(self):
        super().__init__()
        self.sparql_endpoint = settings.eurlex_sparql_endpoint
        self.cellar_endpoint = settings.eurlex_cellar_endpoint
        self.parser = EURLexParser()
        self.html_parser = HTMLParser()
        self.normalizer = TextNormalizer()
        self.chunker = TextChunker(
            max_tokens=settings.chunk_size,
            overlap_tokens=settings.chunk_overlap
        )
    
    async def crawl(self) -> CrawlStats:
        """Crawl EUR-Lex documents."""
        stats = CrawlStats()
        
        logger.info(f"Starting {self.name} crawl")
        
        try:
            # Query SPARQL endpoint for recent German-language EU legislation
            documents = await self._query_recent_legislation()
            stats.documents_found = len(documents)
            
            logger.info(f"Found {len(documents)} EU documents")
            
            # Process each document
            for i, doc_metadata in enumerate(documents):
                try:
                    await self._process_document(doc_metadata, stats)
                    
                    if (i + 1) % 10 == 0:
                        self.log_progress(i + 1, len(documents))
                    
                    # Rate limiting
                    await asyncio.sleep(0.5)
                
                except Exception as e:
                    stats.documents_failed += 1
                    error_msg = f"Failed to process {doc_metadata.get('celex', 'unknown')}: {e}"
                    stats.errors.append(error_msg)
                    logger.error(error_msg)
            
            logger.info(f"EUR-Lex crawl completed: {stats}")
        
        except Exception as e:
            logger.error(f"EUR-Lex crawl failed: {e}")
            stats.errors.append(str(e))
            raise
        
        return stats
    
    async def _query_recent_legislation(self) -> List[Dict[str, Any]]:
        """
        Query SPARQL endpoint for recent legislation.
        
        Returns list of document metadata with CELEX numbers.
        """
        # SPARQL query for German-language EU regulations and directives
        sparql_query = """
        PREFIX cdm: <http://publications.europa.eu/ontology/cdm#>
        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
        PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
        
        SELECT DISTINCT ?work ?celex ?title ?type ?date
        WHERE {
            ?work cdm:work_id_document ?celex .
            ?work cdm:resource_legal_type ?type .
            ?work cdm:work_date_document ?date .
            ?work cdm:expression_title ?title .
            
            FILTER (lang(?title) = "de")
            FILTER (?type IN (
                <http://publications.europa.eu/resource/authority/resource-type/REG>,
                <http://publications.europa.eu/resource/authority/resource-type/DIR>
            ))
            FILTER (?date >= "2020-01-01"^^xsd:date)
        }
        ORDER BY DESC(?date)
        LIMIT 100
        """
        
        try:
            # Query SPARQL endpoint
            response = await self.fetch(
                self.sparql_endpoint,
                method='POST',
                headers={
                    'Accept': 'application/sparql-results+json',
                    'Content-Type': 'application/sparql-query'
                },
                data=sparql_query.encode('utf-8')
            )
            
            json_data = await response.json()
            results = self.parser.parse_sparql_results(json_data)
            
            return results
        
        except Exception as e:
            logger.warning(f"SPARQL query failed: {e}")
            # Fallback to sample data for testing
            return self._get_sample_documents()
    
    def _get_sample_documents(self) -> List[Dict[str, Any]]:
        """Return sample EU documents for testing."""
        return [
            {
                'celex': '32016R0679',  # GDPR
                'title': 'Datenschutz-Grundverordnung',
                'type': 'regulation',
            },
            {
                'celex': '32019R0881',  # Cybersecurity Act
                'title': 'Cybersicherheitsrechtsakt',
                'type': 'regulation',
            },
        ]
    
    async def _process_document(self, doc_metadata: Dict[str, Any], stats: CrawlStats):
        """Process a single EUR-Lex document."""
        celex = doc_metadata.get('celex', '')
        title = doc_metadata.get('title', '')
        
        if not celex:
            logger.warning("No CELEX number found")
            return
        
        # Construct document URL
        doc_url = self._build_document_url(celex)
        
        # Fetch HTML
        try:
            html_content = await self.fetch_text(doc_url)
        except Exception as e:
            logger.warning(f"Failed to fetch {doc_url}: {e}")
            return
        
        # Parse HTML
        parsed = self.parser.parse_eurlex_html(html_content)
        
        # Use parsed title if available
        if parsed.get('title'):
            title = parsed['title']
        
        full_text = parsed.get('full_text', '')
        
        if not full_text:
            logger.warning(f"No content found for {celex}")
            return
        
        # Normalize text
        full_text = self.normalizer.normalize(full_text)
        
        # Generate source ID
        source_id = f"eurlex_{celex}"
        
        # Determine document type
        doc_type = self._determine_document_type(doc_metadata, celex)
        
        # Check if document needs update
        async with await db.get_session() as session:
            doc_ops = DocumentOperations(session)
            content_hash = doc_ops.calculate_content_hash(full_text)
            needs_update, existing_doc = await doc_ops.document_needs_update(
                source_id, content_hash
            )
            
            if not needs_update:
                stats.documents_unchanged += 1
                logger.debug(f"Document unchanged: {celex}")
                return
            
            # Extract metadata
            metadata = {
                'celex': celex,
                'sections': parsed.get('sections', []),
                'document_type_detail': doc_metadata.get('type', ''),
            }
            
            # Create or update document
            if existing_doc:
                document = await doc_ops.update_document(
                    existing_doc,
                    full_text=full_text,
                    title=title,
                    source_url=doc_url,
                    metadata=metadata,
                )
                stats.documents_updated += 1
                logger.info(f"Updated: {celex} - {title}")
            else:
                document = await doc_ops.create_document(
                    source=SourceType.EURLEX,
                    source_id=source_id,
                    title=title,
                    document_type=doc_type,
                    full_text=full_text,
                    jurisdiction='EU',
                    in_force=True,
                    source_url=doc_url,
                    metadata=metadata,
                )
                stats.documents_new += 1
                logger.info(f"Created: {celex} - {title}")
            
            # Generate chunks with section metadata
            sections = parsed.get('sections', [])
            if sections:
                chunks_data = self.chunker.chunk_with_sections(
                    sections,
                    base_metadata={'document_title': title, 'celex': celex}
                )
            else:
                chunks_data = self.chunker.chunk_document(
                    full_text,
                    metadata={'document_title': title, 'celex': celex}
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
            
            logger.debug(f"Created {len(chunks)} chunks for {celex}")
    
    def _build_document_url(self, celex: str) -> str:
        """Build URL for EUR-Lex document."""
        # Format: https://eur-lex.europa.eu/legal-content/DE/TXT/HTML/?uri=CELEX:32016R0679
        return f"{self.cellar_endpoint}/DE/TXT/HTML/?uri=CELEX:{celex}"
    
    def _determine_document_type(
        self,
        metadata: Dict[str, Any],
        celex: str
    ) -> DocumentType:
        """Determine document type from metadata or CELEX number."""
        doc_type = metadata.get('type', '').lower()
        
        # Check type from metadata
        if 'regulation' in doc_type or 'reg' in doc_type:
            return DocumentType.EU_VERORDNUNG
        elif 'directive' in doc_type or 'dir' in doc_type:
            return DocumentType.EU_RICHTLINIE
        elif 'decision' in doc_type:
            return DocumentType.EU_BESCHLUSS
        
        # Parse CELEX number (format: 32016R0679)
        # Letter indicates type: R=Regulation, L=Directive, D=Decision
        if len(celex) >= 6:
            type_letter = celex[5]
            if type_letter == 'R':
                return DocumentType.EU_VERORDNUNG
            elif type_letter == 'L':
                return DocumentType.EU_RICHTLINIE
            elif type_letter == 'D':
                return DocumentType.EU_BESCHLUSS
        
        # Default to Verordnung
        return DocumentType.EU_VERORDNUNG
