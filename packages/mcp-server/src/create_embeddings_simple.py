#!/usr/bin/env python3
"""
Simple script to create law_chunks with embeddings.
"""
import asyncio
import os
from sqlalchemy import select
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession, async_sessionmaker
from sqlalchemy.orm import declarative_base
import sys

# Database URL
DATABASE_URL = os.getenv("POSTGRES_URL", "postgresql+asyncpg://legal:legal_dev_password@postgres:5432/legalmcp")

# Import models after setting up path
sys.path.insert(0, '/app')
from src.db.models import Law, LawChunk
from src.search.embeddings import get_embedding_service
from uuid import uuid4


async def main():
    """Main function."""
    print("🚀 Starting embedding generation...\n")
    
    # Create engine and session
    engine = create_async_engine(DATABASE_URL, echo=False)
    async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)
    
    async with async_session() as session:
        # Get all laws
        stmt = select(Law)
        result = await session.execute(stmt)
        laws = result.scalars().all()
        
        print(f"Found {len(laws)} laws\n")
        
        embedding_service = get_embedding_service()
        
        for law in laws:
            print(f"Processing: {law.title}")
            
            if not law.full_text:
                print("  ⚠️  No text\n")
                continue
            
            # Split by paragraphs
            parts = [p.strip() for p in law.full_text.split('\n\n') if p.strip() and len(p.strip()) > 50]
            
            for idx, part in enumerate(parts):
                # Extract section
                section = None
                if part.startswith(('Artikel', 'Article', 'Art.')):
                    section = part.split('\n')[0].split('-')[0].strip()
                
                # Generate embedding
                try:
                    embedding = await embedding_service.encode_text(part)
                    
                    chunk = LawChunk(
                        id=uuid4(),
                        law_id=law.id,
                        chunk_index=idx,
                        section_reference=section,
                        content=part[:2000],  # Limit content length
                        embedding=embedding,
                        metadata={}
                    )
                    
                    session.add(chunk)
                    print(f"  ✓ Chunk {idx + 1}: {section or 'general'}")
                    
                except Exception as e:
                    print(f"  ✗ Error: {e}")
            
            await session.commit()
            print(f"  ✅ Done\n")
        
        print("✅ All laws processed!")
    
    await engine.dispose()


if __name__ == "__main__":
    asyncio.run(main())
