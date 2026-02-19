#!/usr/bin/env python3
"""
Script to create law_chunks with embeddings for all laws in the database.
This script chunks the full_text of each law and generates embeddings.
"""
import asyncio
import sys
import os

# Add parent directory to path to import from src
sys.path.insert(0, '/app')

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from src.db.session import get_db
from src.db.models import Law, LawChunk
from src.search.embeddings import get_embedding_service
from uuid import uuid4


async def chunk_text(text: str, chunk_size: int = 500, overlap: int = 50) -> list[str]:
    """Simple text chunking by characters."""
    chunks = []
    start = 0
    while start < len(text):
        end = start + chunk_size
        chunk = text[start:end]
        if chunk.strip():
            chunks.append(chunk.strip())
        start = end - overlap
    return chunks


async def create_chunks_for_law(db: AsyncSession, law: Law):
    """Create chunks with embeddings for a single law."""
    print(f"Processing: {law.title} ({law.abbreviation})")
    
    if not law.full_text:
        print(f"  ⚠️  No full_text available, skipping...")
        return
    
    # Split text into articles/sections
    text_parts = law.full_text.split('\n\n')
    
    embedding_service = get_embedding_service()
    chunk_index = 0
    
    for part in text_parts:
        if not part.strip() or len(part.strip()) < 50:
            continue
            
        # Extract section reference if it starts with "Artikel" or "Article"
        section_ref = None
        if part.strip().startswith(('Artikel', 'Article', 'Art.')):
            # Extract the article number
            first_line = part.strip().split('\n')[0]
            section_ref = first_line.split('-')[0].strip()
        
        # Generate embedding
        try:
            embedding = await embedding_service.encode_text(part)
            
            # Create chunk
            chunk = LawChunk(
                id=uuid4(),
                law_id=law.id,
                chunk_index=chunk_index,
                section_reference=section_ref,
                content=part.strip(),
                embedding=embedding,
                metadata={}
            )
            
            db.add(chunk)
            chunk_index += 1
            print(f"  ✓ Created chunk {chunk_index}: {section_ref or 'no section'}")
            
        except Exception as e:
            print(f"  ✗ Error creating chunk: {e}")
            continue
    
    await db.commit()
    print(f"  ✅ Created {chunk_index} chunks for {law.abbreviation}\n")


async def main():
    """Main function to process all laws."""
    print("🚀 Starting law chunking and embedding generation...\n")
    
    # Get database session
    async for db in get_db():
        try:
            # Get all laws
            stmt = select(Law)
            result = await db.execute(stmt)
            laws = result.scalars().all()
            
            print(f"Found {len(laws)} laws in database\n")
            
            # Process each law
            for law in laws:
                await create_chunks_for_law(db, law)
            
            print("\n✅ All laws processed successfully!")
            
        except Exception as e:
            print(f"\n❌ Error: {e}")
            import traceback
            traceback.print_exc()
        
        # Only process once
        return


if __name__ == "__main__":
    asyncio.run(main())
