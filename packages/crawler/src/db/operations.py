"""Database operations and utilities."""

from typing import List, Optional, Dict, Any
from datetime import datetime
import hashlib
from sqlalchemy import select, update, and_
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine, async_sessionmaker
from sqlalchemy.orm import selectinload

from .models import (
    Base, Law, LawChunk, CaseLaw, CaseLawChunk, CrawlRun, 
    SourceType, DocumentType, JurisdictionType
)
from ..config import settings


class Database:
    """Database manager for async operations."""
    
    def __init__(self):
        """Initialize database connection."""
        self.engine = create_async_engine(
            settings.database_url,
            echo=False,
            pool_pre_ping=True,
            pool_size=10,
            max_overflow=20,
        )
        self.async_session = async_sessionmaker(
            self.engine,
            class_=AsyncSession,
            expire_on_commit=False,
        )
    
    async def init_db(self):
        """Initialize database tables."""
        async with self.engine.begin() as conn:
            await conn.run_sync(Base.metadata.create_all)
    
    async def close(self):
        """Close database connection."""
        await self.engine.dispose()
    
    async def get_session(self) -> AsyncSession:
        """Get async session."""
        return self.async_session()


class LawOperations:
    """Operations for law/document management."""
    
    def __init__(self, session: AsyncSession):
        self.session = session
    
    @staticmethod
    def calculate_content_hash(text: str) -> str:
        """Calculate SHA-256 hash of content."""
        return hashlib.sha256(text.encode('utf-8')).hexdigest()
    
    async def get_law_by_source_url(self, source_url: str) -> Optional[Law]:
        """Get law by source URL."""
        result = await self.session.execute(
            select(Law).where(Law.source_url == source_url)
        )
        return result.scalar_one_or_none()
    
    async def law_needs_update(
        self, 
        source_url: str, 
        new_content_hash: str
    ) -> tuple[bool, Optional[Law]]:
        """
        Check if law needs update based on content hash.
        
        Returns:
            (needs_update, existing_law)
        """
        law = await self.get_law_by_source_url(source_url)
        if law is None:
            return True, None
        
        # Check content hash stored in metadata
        old_hash = law.metadata.get('content_hash', '')
        return old_hash != new_content_hash, law
    
    async def create_law(
        self,
        title: str,
        jurisdiction: JurisdictionType,
        document_type: DocumentType,
        source_url: str,
        full_text: str,
        source: SourceType,
        **kwargs
    ) -> Law:
        """Create a new law."""
        content_hash = self.calculate_content_hash(full_text)
        
        # Store source and content_hash in metadata
        metadata = kwargs.pop('metadata', {})
        metadata['source'] = source.value
        metadata['content_hash'] = content_hash
        
        law = Law(
            title=title,
            jurisdiction=jurisdiction,
            document_type=document_type,
            source_url=source_url,
            full_text=full_text,
            metadata=metadata,
            **kwargs
        )
        
        self.session.add(law)
        await self.session.flush()
        return law
    
    async def update_law(
        self,
        law: Law,
        full_text: str,
        **kwargs
    ) -> Law:
        """Update an existing law."""
        law.full_text = full_text
        
        # Update content hash in metadata
        content_hash = self.calculate_content_hash(full_text)
        law.metadata['content_hash'] = content_hash
        
        for key, value in kwargs.items():
            if hasattr(law, key):
                setattr(law, key, value)
        
        await self.session.flush()
        return law
    
    async def delete_law_chunks(self, law_id):
        """Delete all chunks for a law."""
        await self.session.execute(
            LawChunk.__table__.delete().where(
                LawChunk.law_id == law_id
            )
        )
    
    async def create_chunks(
        self,
        law_id,
        chunks_data: List[Dict[str, Any]]
    ) -> List[LawChunk]:
        """Create multiple chunks for a law."""
        chunks = []
        for i, chunk_data in enumerate(chunks_data):
            chunk = LawChunk(
                law_id=law_id,
                chunk_index=i,
                **chunk_data
            )
            chunks.append(chunk)
            self.session.add(chunk)
        
        await self.session.flush()
        return chunks
    
    async def update_chunk_embedding(
        self,
        chunk_id,
        embedding: List[float]
    ):
        """Update embedding for a chunk."""
        await self.session.execute(
            update(LawChunk)
            .where(LawChunk.id == chunk_id)
            .values(embedding=embedding)
        )
    
    async def commit(self):
        """Commit transaction."""
        await self.session.commit()
    
    async def rollback(self):
        """Rollback transaction."""
        await self.session.rollback()


class CrawlRunOperations:
    """Operations for crawl run logging."""
    
    def __init__(self, session: AsyncSession):
        self.session = session
    
    async def create_run(
        self,
        source: str,
        status: str = "running"
    ) -> CrawlRun:
        """Create a new crawl run entry."""
        run = CrawlRun(
            source=source,
            status=status,
            started_at=datetime.utcnow()
        )
        self.session.add(run)
        await self.session.flush()
        return run
    
    async def update_run(
        self,
        run: CrawlRun,
        status: str,
        **kwargs
    ):
        """Update crawl run."""
        run.status = status
        
        if status in ["completed", "failed"]:
            run.completed_at = datetime.utcnow()
        
        for key, value in kwargs.items():
            if hasattr(run, key):
                setattr(run, key, value)
        
        await self.session.flush()
    
    async def commit(self):
        """Commit transaction."""
        await self.session.commit()


# Global database instance
db = Database()
