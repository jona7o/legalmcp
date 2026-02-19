"""Base crawler interface."""

import logging
from abc import ABC, abstractmethod
from typing import Optional, Dict, Any, List
from datetime import datetime
import aiohttp
from aiolimiter import AsyncLimiter
from tenacity import (
    retry,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception_type
)

from ..config import settings
from ..db import SourceType, CrawlRun

logger = logging.getLogger(__name__)


class BaseCrawler(ABC):
    """Abstract base class for legal document crawlers."""
    
    def __init__(self):
        """Initialize crawler."""
        self.session: Optional[aiohttp.ClientSession] = None
        self.rate_limiter = AsyncLimiter(
            max_rate=settings.crawler_rate_limit,
            time_period=1.0
        )
        self.user_agent = settings.crawler_user_agent
        self.timeout = aiohttp.ClientTimeout(total=settings.crawler_timeout)
    
    @property
    @abstractmethod
    def source_type(self) -> SourceType:
        """Return the source type for this crawler."""
        pass
    
    @property
    @abstractmethod
    def name(self) -> str:
        """Return the crawler name."""
        pass
    
    async def __aenter__(self):
        """Async context manager entry."""
        await self.start()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()
    
    async def start(self):
        """Initialize crawler session."""
        if self.session is None:
            self.session = aiohttp.ClientSession(
                headers={'User-Agent': self.user_agent},
                timeout=self.timeout
            )
        logger.info(f"{self.name} crawler started")
    
    async def close(self):
        """Close crawler session."""
        if self.session:
            await self.session.close()
            self.session = None
        logger.info(f"{self.name} crawler closed")
    
    @retry(
        stop=stop_after_attempt(settings.crawler_retry_attempts),
        wait=wait_exponential(multiplier=1, min=2, max=10),
        retry=retry_if_exception_type((aiohttp.ClientError, TimeoutError)),
        reraise=True
    )
    async def fetch(
        self,
        url: str,
        method: str = 'GET',
        **kwargs
    ) -> aiohttp.ClientResponse:
        """
        Fetch URL with rate limiting and retry logic.
        
        Args:
            url: URL to fetch
            method: HTTP method
            **kwargs: Additional arguments for request
            
        Returns:
            Response object
        """
        if not self.session:
            raise RuntimeError("Crawler session not initialized. Use 'async with' context manager.")
        
        async with self.rate_limiter:
            logger.debug(f"Fetching: {url}")
            async with self.session.request(method, url, **kwargs) as response:
                response.raise_for_status()
                return response
    
    async def fetch_text(self, url: str, encoding: str = 'utf-8', **kwargs) -> str:
        """
        Fetch URL and return text content.
        
        Args:
            url: URL to fetch
            encoding: Text encoding
            **kwargs: Additional arguments for request
            
        Returns:
            Response text
        """
        response = await self.fetch(url, **kwargs)
        return await response.text(encoding=encoding)
    
    async def fetch_json(self, url: str, **kwargs) -> Dict[str, Any]:
        """
        Fetch URL and return JSON content.
        
        Args:
            url: URL to fetch
            **kwargs: Additional arguments for request
            
        Returns:
            Parsed JSON
        """
        response = await self.fetch(url, **kwargs)
        return await response.json()
    
    async def check_robots_txt(self, base_url: str) -> bool:
        """
        Check robots.txt for crawling permission.
        
        Args:
            base_url: Base URL of the site
            
        Returns:
            True if crawling is allowed
        """
        try:
            robots_url = f"{base_url.rstrip('/')}/robots.txt"
            text = await self.fetch_text(robots_url)
            
            # Simple check - look for our user agent or general disallow
            if 'User-agent: *' in text:
                if 'Disallow: /' in text:
                    logger.warning(f"robots.txt disallows crawling: {base_url}")
                    return False
            
            return True
        
        except Exception as e:
            logger.debug(f"Could not fetch robots.txt: {e}")
            # If robots.txt doesn't exist, assume crawling is allowed
            return True
    
    @abstractmethod
    async def crawl(self) -> CrawlStats:
        """
        Execute the crawl process.
        
        Returns:
            Statistics about the crawl
        """
        pass
    
    def log_progress(self, current: int, total: int, message: str = ""):
        """Log progress of crawling."""
        percentage = (current / total * 100) if total > 0 else 0
        logger.info(f"{self.name}: {current}/{total} ({percentage:.1f}%) {message}")


class CrawlStats:
    """Statistics for a crawl run."""
    
    def __init__(self):
        self.documents_found = 0
        self.documents_new = 0
        self.documents_updated = 0
        self.documents_unchanged = 0
        self.documents_failed = 0
        self.errors: List[str] = []
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            'documents_found': self.documents_found,
            'documents_new': self.documents_new,
            'documents_updated': self.documents_updated,
            'documents_unchanged': self.documents_unchanged,
            'documents_failed': self.documents_failed,
            'error_count': len(self.errors),
        }
    
    def __repr__(self) -> str:
        return (
            f"CrawlStats(found={self.documents_found}, new={self.documents_new}, "
            f"updated={self.documents_updated}, unchanged={self.documents_unchanged}, "
            f"failed={self.documents_failed})"
        )
