"""Vector store operations using pgvector."""

from typing import List, Optional
from uuid import UUID

from sqlalchemy import select, func, literal_column
from sqlalchemy.ext.asyncio import AsyncSession

from src.config import get_settings
from src.db.models import CaseLawChunk, LawChunk
from src.search.embeddings import get_embedding_service

settings = get_settings()


class VectorStore:
    """Vector store for semantic search using pgvector."""

    def __init__(self, session: AsyncSession):
        """
        Initialize vector store.

        Args:
            session: Database session
        """
        self.session = session
        self.embedding_service = get_embedding_service()

    async def search_laws(
        self,
        query: str,
        limit: int = 10,
        min_score: Optional[float] = None,
        jurisdiction_filter: Optional[List[str]] = None,
        document_type_filter: Optional[List[str]] = None,
    ) -> List[dict]:
        """
        Search for laws using semantic similarity.

        Args:
            query: Search query
            limit: Maximum number of results
            min_score: Minimum similarity score (0-1)
            jurisdiction_filter: Filter by jurisdiction types
            document_type_filter: Filter by document types

        Returns:
            List of law chunks with similarity scores
        """
        # Generate query embedding
        query_embedding = await self.embedding_service.encode_text(query)
        
        # Convert embedding list to pgvector string format: '[1.0, 2.0, ...]'
        embedding_str = '[' + ','.join(str(x) for x in query_embedding) + ']'
        
        # Build query with cosine similarity using pgvector operator
        # Use literal_column to pass the vector as SQL literal with cast
        query_vector = literal_column(f"'{embedding_str}'::vector(768)")
        similarity_expr = (1 - LawChunk.embedding.op("<=>")(query_vector)).label("similarity")
        
        stmt = (
            select(
                LawChunk.id,
                LawChunk.law_id,
                LawChunk.chunk_index,
                LawChunk.section_reference,
                LawChunk.content,
                LawChunk.meta,  # Column is named 'meta' not 'metadata'
                similarity_expr,
            )
            .join(LawChunk.law)
            .order_by(similarity_expr.desc())
            .limit(limit)
        )

        # Apply filters
        if jurisdiction_filter:
            from src.db.models import Law
            stmt = stmt.where(Law.jurisdiction.in_(jurisdiction_filter))

        if document_type_filter:
            from src.db.models import Law
            stmt = stmt.where(Law.document_type.in_(document_type_filter))

        if min_score:
            stmt = stmt.where(similarity_expr >= min_score)

        result = await self.session.execute(stmt)
        rows = result.all()

        return [
            {
                "chunk_id": str(row.id),
                "law_id": str(row.law_id),
                "content": row.content,
                "section_reference": row.section_reference,
                "similarity": float(row.similarity),
                "metadata": row.meta,  # Map back to 'metadata' in response
            }
            for row in rows
        ]

    async def search_case_law(
        self,
        query: str,
        limit: int = 10,
        min_score: Optional[float] = None,
        jurisdiction_filter: Optional[List[str]] = None,
        court_level_filter: Optional[List[str]] = None,
    ) -> List[dict]:
        """
        Search for case law using semantic similarity.

        Args:
            query: Search query
            limit: Maximum number of results
            min_score: Minimum similarity score
            jurisdiction_filter: Filter by jurisdiction
            court_level_filter: Filter by court level

        Returns:
            List of case law chunks with similarity scores
        """
        # Generate query embedding
        query_embedding = await self.embedding_service.encode_text(query)
        
        # Convert to pgvector format
        embedding_str = '[' + ','.join(str(x) for x in query_embedding) + ']'
        query_vector = literal_column(f"'{embedding_str}'::vector(768)")
        similarity_expr = (1 - CaseLawChunk.embedding.op("<=>")(query_vector)).label("similarity")
        
        stmt = (
            select(
                CaseLawChunk.id,
                CaseLawChunk.case_law_id,
                CaseLawChunk.chunk_index,
                CaseLawChunk.content,
                CaseLawChunk.meta,  # Column is named 'meta'
                similarity_expr,
            )
            .join(CaseLawChunk.case_law)
            .order_by(similarity_expr.desc())
            .limit(limit)
        )

        # Apply filters
        if jurisdiction_filter:
            from src.db.models import CaseLaw
            stmt = stmt.where(CaseLaw.jurisdiction.in_(jurisdiction_filter))

        if court_level_filter:
            from src.db.models import CaseLaw
            stmt = stmt.where(CaseLaw.court_level.in_(court_level_filter))

        if min_score:
            stmt = stmt.where(similarity_expr >= min_score)

        result = await self.session.execute(stmt)
        rows = result.all()

        return [
            {
                "chunk_id": str(row.id),
                "case_law_id": str(row.case_law_id),
                "content": row.content,
                "similarity": float(row.similarity),
                "metadata": row.meta,  # Map back to 'metadata'
            }
            for row in rows
        ]

    async def get_similar_laws(
        self,
        law_id: UUID,
        limit: int = 5,
    ) -> List[dict]:
        """
        Find laws similar to a given law.

        Args:
            law_id: UUID of the law
            limit: Maximum number of results

        Returns:
            List of similar laws
        """
        # Get embeddings for the law - select only the embedding column
        stmt = select(LawChunk.id, LawChunk.embedding).where(LawChunk.law_id == law_id).limit(1)
        result = await self.session.execute(stmt)
        row = result.first()

        if not row:
            return []

        # Use the first chunk's embedding as representative
        reference_embedding = row.embedding

        # Convert to pgvector format
        embedding_str = '[' + ','.join(str(x) for x in reference_embedding) + ']'
        ref_vector = literal_column(f"'{embedding_str}'::vector(768)")
        similarity_expr = (1 - LawChunk.embedding.op("<=>")(ref_vector)).label("similarity")
        
        stmt = (
            select(
                LawChunk.id,
                LawChunk.law_id,
                LawChunk.content,
                LawChunk.section_reference,
                similarity_expr,
            )
            .where(LawChunk.law_id != law_id)  # Exclude the same law
            .order_by(similarity_expr.desc())
            .limit(limit)
        )

        result = await self.session.execute(stmt)
        rows = result.all()

        return [
            {
                "chunk_id": str(row.id),
                "law_id": str(row.law_id),
                "content": row.content,
                "similarity": float(row.similarity),
            }
            for row in rows
        ]
