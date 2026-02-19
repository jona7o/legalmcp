"""
Focused EUR-Lex Crawler for GDPR and AI Act.

This crawler specifically targets:
- GDPR (Regulation 2016/679) - CELEX: 32016R0679
- AI Act (Regulation 2024/1689) - CELEX: 32024R1689
"""

import asyncio
import logging
from typing import List, Dict, Any
from datetime import datetime

from ..config import settings
from ..db.operations import Database, LawOperations, CrawlRunOperations
from ..db.models import JurisdictionType, DocumentType, SourceType
from ..processing.parser import EURLexParser
from ..processing.normalizer import TextNormalizer
from ..processing.chunker import TextChunker
from ..processing.embedder import EmbeddingGenerator

logger = logging.getLogger(__name__)


class FocusedEURLexCrawler:
    """Crawler for specific EU regulations."""
    
    # Target documents
    TARGETS = [
        {
            "celex": "32016R0679",
            "title": "Datenschutz-Grundverordnung (DSGVO)",
            "abbreviation": "DSGVO",
            "url": "https://eur-lex.europa.eu/legal-content/DE/TXT/HTML/?uri=CELEX:32016R0679"
        },
        {
            "celex": "32024R1689",
            "title": "Verordnung über Künstliche Intelligenz (AI Act)",
            "abbreviation": "AI Act",
            "url": "https://eur-lex.europa.eu/legal-content/DE/TXT/HTML/?uri=CELEX:32024R1689"
        }
    ]
    
    def __init__(self):
        self.parser = EURLexParser()
        self.normalizer = TextNormalizer()
        self.chunker = TextChunker()
        self.embedder = EmbeddingGenerator()
        
    async def crawl_document(
        self, 
        target: Dict[str, str],
        db: Database
    ) -> Dict[str, Any]:
        """
        Crawl a single EU document.
        
        Args:
            target: Document metadata (celex, title, url)
            db: Database instance
            
        Returns:
            Crawl statistics
        """
        stats = {
            "celex": target["celex"],
            "title": target["title"],
            "success": False,
            "chunks_created": 0,
            "error": None
        }
        
        try:
            logger.info(f"📄 Crawling: {target['title']}")
            
            # Fetch content
            content = await self.parser.fetch_url(target["url"])
            parsed_data = await self.parser.parse_html(content)
            
            if not parsed_data or not parsed_data.get("full_text"):
                stats["error"] = "No content found"
                return stats
            
            # Normalize text
            full_text = self.normalizer.normalize(parsed_data["full_text"])
            
            # Get or create database session
            async with db.get_session() as session:
                law_ops = LawOperations(session)
                
                # Check if document exists
                needs_update, existing_law = await law_ops.law_needs_update(
                    source_url=target["url"],
                    new_content_hash=law_ops.calculate_content_hash(full_text)
                )
                
                if not needs_update and existing_law:
                    logger.info(f"✓ {target['title']} already up to date")
                    stats["success"] = True
                    return stats
                
                # Create or update law
                if existing_law:
                    logger.info(f"🔄 Updating: {target['title']}")
                    law = await law_ops.update_law(
                        law=existing_law,
                        full_text=full_text
                    )
                    # Delete old chunks
                    await law_ops.delete_law_chunks(law.id)
                else:
                    logger.info(f"✨ Creating: {target['title']}")
                    law = await law_ops.create_law(
                        title=target["title"],
                        abbreviation=target.get("abbreviation"),
                        jurisdiction=JurisdictionType.EU,
                        document_type=DocumentType.REGULATION,
                        source_url=target["url"],
                        full_text=full_text,
                        source=SourceType.EURLEX,
                        official_number=target["celex"],
                        metadata={
                            "celex": target["celex"],
                            "language": "DE"
                        }
                    )
                
                # Create chunks
                logger.info(f"📝 Chunking text...")
                chunks_data = await self.chunker.chunk_text(
                    text=full_text,
                    metadata={
                        "law_id": str(law.id),
                        "title": law.title
                    }
                )
                
                logger.info(f"📦 Created {len(chunks_data)} chunks")
                
                # Prepare chunks for database
                db_chunks_data = []
                for chunk_data in chunks_data:
                    db_chunks_data.append({
                        "content": chunk_data["text"],
                        "section_reference": chunk_data.get("metadata", {}).get("section"),
                        "metadata": chunk_data.get("metadata", {})
                    })
                
                # Insert chunks
                chunks = await law_ops.create_chunks(
                    law_id=law.id,
                    chunks_data=db_chunks_data
                )
                
                logger.info(f"🧠 Generating embeddings...")
                
                # Generate embeddings
                texts = [chunk_data["text"] for chunk_data in chunks_data]
                embeddings = await self.embedder.generate_embeddings(texts)
                
                # Update chunks with embeddings
                for chunk, embedding in zip(chunks, embeddings):
                    await law_ops.update_chunk_embedding(
                        chunk_id=chunk.id,
                        embedding=embedding.tolist() if hasattr(embedding, 'tolist') else embedding
                    )
                
                # Commit transaction
                await law_ops.commit()
                
                stats["success"] = True
                stats["chunks_created"] = len(chunks)
                logger.info(f"✅ {target['title']}: {len(chunks)} chunks with embeddings")
                
        except Exception as e:
            logger.error(f"❌ Error crawling {target['title']}: {e}", exc_info=True)
            stats["error"] = str(e)
        
        return stats
    
    async def run(self):
        """Run the focused crawler."""
        logger.info("🚀 Starting Focused EUR-Lex Crawler")
        logger.info(f"📋 Target documents: {len(self.TARGETS)}")
        
        # Initialize database
        db = Database()
        await db.init_db()
        
        # Create crawl run
        async with db.get_session() as session:
            crawl_ops = CrawlRunOperations(session)
            crawl_run = await crawl_ops.create_run(
                source="eurlex-focused",
                status="running"
            )
            await crawl_ops.commit()
        
        crawl_run_id = crawl_run.id
        
        # Crawl each document
        all_stats = []
        for target in self.TARGETS:
            stats = await self.crawl_document(target, db)
            all_stats.append(stats)
            
            # Small delay between requests
            await asyncio.sleep(2)
        
        # Update crawl run
        async with db.get_session() as session:
            crawl_ops = CrawlRunOperations(session)
            
            # Get crawl run
            from sqlalchemy import select
            from ..db.models import CrawlRun
            stmt = select(CrawlRun).where(CrawlRun.id == crawl_run_id)
            result = await session.execute(stmt)
            crawl_run = result.scalar_one()
            
            successful = sum(1 for s in all_stats if s["success"])
            failed = len(all_stats) - successful
            total_chunks = sum(s.get("chunks_created", 0) for s in all_stats)
            
            await crawl_ops.update_run(
                run=crawl_run,
                status="completed",
                documents_processed=len(all_stats),
                documents_created=successful,
                errors_count=failed,
                metadata={
                    "total_chunks": total_chunks,
                    "details": all_stats
                }
            )
            await crawl_ops.commit()
        
        await db.close()
        
        # Print summary
        logger.info("\n" + "="*60)
        logger.info("📊 CRAWL SUMMARY")
        logger.info("="*60)
        for stats in all_stats:
            status = "✅" if stats["success"] else "❌"
            logger.info(f"{status} {stats['title']}")
            if stats["success"]:
                logger.info(f"   Chunks: {stats['chunks_created']}")
            elif stats["error"]:
                logger.info(f"   Error: {stats['error']}")
        logger.info("="*60)
        logger.info(f"Total: {successful}/{len(all_stats)} successful")
        logger.info(f"Total chunks: {total_chunks}")
        logger.info("="*60)
        
        return all_stats


async def main():
    """Main entry point for focused crawler."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    
    crawler = FocusedEURLexCrawler()
    await crawler.run()


if __name__ == "__main__":
    asyncio.run(main())
