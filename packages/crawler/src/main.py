"""Main crawler orchestrator."""

import asyncio
import logging
import sys
from datetime import datetime
from typing import List, Dict

from .config import settings
from .db import db, CrawlRunOperations, SourceType
from .sources import BundesrechtCrawler, BayernCrawler, EURLexCrawler, CrawlStats

# Configure logging
logging.basicConfig(
    level=getattr(logging, settings.log_level.upper()),
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout),
        logging.FileHandler('crawler.log')
    ]
)

logger = logging.getLogger(__name__)


class CrawlerOrchestrator:
    """Main orchestrator for all crawlers."""
    
    def __init__(self):
        self.crawlers = []
        
        # Initialize enabled crawlers
        if settings.enable_bundesrecht:
            self.crawlers.append(BundesrechtCrawler())
        
        if settings.enable_bayern:
            self.crawlers.append(BayernCrawler())
        
        if settings.enable_eurlex:
            self.crawlers.append(EURLexCrawler())
    
    async def run_all(self) -> Dict[str, CrawlStats]:
        """Run all enabled crawlers."""
        logger.info("=== Starting Legal MCP Crawler ===")
        logger.info(f"Enabled crawlers: {[c.name for c in self.crawlers]}")
        
        # Initialize database
        await db.init_db()
        logger.info("Database initialized")
        
        results = {}
        
        for crawler in self.crawlers:
            try:
                logger.info(f"\n{'='*60}")
                logger.info(f"Starting crawler: {crawler.name}")
                logger.info(f"{'='*60}\n")
                
                stats = await self.run_crawler(crawler)
                results[crawler.name] = stats
                
                logger.info(f"\n{crawler.name} completed: {stats}\n")
            
            except Exception as e:
                logger.error(f"Crawler {crawler.name} failed: {e}", exc_info=True)
                results[crawler.name] = None
        
        # Print summary
        self._print_summary(results)
        
        return results
    
    async def run_crawler(self, crawler) -> CrawlStats:
        """Run a single crawler with logging."""
        start_time = datetime.utcnow()
        
        # Create crawl log
        async with await db.get_session() as session:
            log_ops = CrawlRunOperations(session)
            crawl_log = await log_ops.create_log(
                source=crawler.source_type,
                status="running"
            )
            await log_ops.commit()
            log_id = crawl_log.id
        
        try:
            # Run crawler
            async with crawler:
                stats = await crawler.crawl()
            
            # Update log as completed
            async with await db.get_session() as session:
                log_ops = CrawlRunOperations(session)
                # Re-fetch the log
                from sqlalchemy import select
                from .db.models import CrawlRun
                result = await session.execute(
                    select(CrawlRun).where(CrawlRun.id == log_id)
                )
                crawl_log = result.scalar_one()
                
                await log_ops.update_log(
                    crawl_log,
                    status="completed",
                    documents_found=stats.documents_found,
                    documents_new=stats.documents_new,
                    documents_updated=stats.documents_updated,
                    documents_unchanged=stats.documents_unchanged,
                    documents_failed=stats.documents_failed,
                )
                await log_ops.commit()
            
            return stats
        
        except Exception as e:
            # Update log as failed
            async with await db.get_session() as session:
                log_ops = CrawlRunOperations(session)
                from sqlalchemy import select
                from .db.models import CrawlRun
                result = await session.execute(
                    select(CrawlRun).where(CrawlRun.id == log_id)
                )
                crawl_log = result.scalar_one()
                
                await log_ops.update_log(
                    crawl_log,
                    status="failed",
                    error_message=str(e)
                )
                await log_ops.commit()
            
            raise
    
    def _print_summary(self, results: Dict[str, CrawlStats]):
        """Print summary of crawl results."""
        logger.info("\n" + "="*60)
        logger.info("CRAWL SUMMARY")
        logger.info("="*60)
        
        total_found = 0
        total_new = 0
        total_updated = 0
        total_unchanged = 0
        total_failed = 0
        
        for name, stats in results.items():
            if stats:
                logger.info(f"\n{name}:")
                logger.info(f"  Found: {stats.documents_found}")
                logger.info(f"  New: {stats.documents_new}")
                logger.info(f"  Updated: {stats.documents_updated}")
                logger.info(f"  Unchanged: {stats.documents_unchanged}")
                logger.info(f"  Failed: {stats.documents_failed}")
                
                total_found += stats.documents_found
                total_new += stats.documents_new
                total_updated += stats.documents_updated
                total_unchanged += stats.documents_unchanged
                total_failed += stats.documents_failed
            else:
                logger.info(f"\n{name}: FAILED")
        
        logger.info("\n" + "-"*60)
        logger.info("TOTAL:")
        logger.info(f"  Found: {total_found}")
        logger.info(f"  New: {total_new}")
        logger.info(f"  Updated: {total_updated}")
        logger.info(f"  Unchanged: {total_unchanged}")
        logger.info(f"  Failed: {total_failed}")
        logger.info("="*60 + "\n")


async def main():
    """Main entry point."""
    try:
        orchestrator = CrawlerOrchestrator()
        await orchestrator.run_all()
        logger.info("Crawler completed successfully")
        return 0
    
    except Exception as e:
        logger.error(f"Crawler failed: {e}", exc_info=True)
        return 1
    
    finally:
        await db.close()


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
