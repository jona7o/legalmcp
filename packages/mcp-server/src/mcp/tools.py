"""MCP Tools - Business logic for MCP tool implementations."""

from typing import Any, Dict, List, Optional
from uuid import UUID

from sqlalchemy import desc, select
from sqlalchemy.ext.asyncio import AsyncSession

from src.db.models import CaseLaw, Law
from src.search.vector_store import VectorStore


class MCPTools:
    """Collection of MCP tool implementations."""

    def __init__(self, session: AsyncSession):
        """
        Initialize MCP tools.

        Args:
            session: Database session
        """
        self.session = session
        self.vector_store = VectorStore(session)

    async def search_laws(
        self,
        query: str,
        jurisdiction: Optional[List[str]] = None,
        document_type: Optional[List[str]] = None,
        include_case_law: bool = False,
        limit: int = 10,
        min_score: Optional[float] = None,
    ) -> Dict[str, Any]:
        """
        Search for laws using semantic similarity.

        Args:
            query: Search query
            jurisdiction: Filter by jurisdictions
            document_type: Filter by document types
            include_case_law: Include case law in results
            limit: Maximum number of results
            min_score: Minimum similarity score

        Returns:
            Dictionary with search results
        """
        results = await self.vector_store.search_laws(
            query=query,
            limit=limit,
            min_score=min_score,
            jurisdiction_filter=jurisdiction,
            document_type_filter=document_type,
        )

        # Fetch law details
        search_results = []
        for result in results:
            stmt = select(Law).where(Law.id == result["law_id"])
            law_result = await self.session.execute(stmt)
            law = law_result.scalar_one_or_none()

            if law:
                search_results.append({
                    "id": str(law.id),
                    "type": "law",
                    "title": law.title,
                    "abbreviation": law.abbreviation,
                    "content": result["content"],
                    "similarity": result["similarity"],
                    "jurisdiction": law.jurisdiction.value,
                    "document_type": law.document_type.value,
                    "section_reference": result.get("section_reference"),
                    "source_url": law.source_url,
                })

        return {
            "query": query,
            "results": search_results,
            "total": len(search_results),
        }

    async def get_law_by_id(
        self,
        law_id: Optional[UUID] = None,
        abbreviation: Optional[str] = None,
    ) -> Optional[Dict[str, Any]]:
        """
        Get law by ID or abbreviation.

        Args:
            law_id: Law UUID
            abbreviation: Law abbreviation

        Returns:
            Law details or None
        """
        if law_id:
            stmt = select(Law).where(Law.id == law_id)
        elif abbreviation:
            stmt = select(Law).where(Law.abbreviation.ilike(abbreviation))
        else:
            return None

        result = await self.session.execute(stmt)
        law = result.scalar_one_or_none()

        if not law:
            return None

        return {
            "id": str(law.id),
            "title": law.title,
            "abbreviation": law.abbreviation,
            "jurisdiction": law.jurisdiction.value,
            "document_type": law.document_type.value,
            "official_number": law.official_number,
            "publication_date": str(law.publication_date) if law.publication_date else None,
            "last_modified_date": str(law.last_modified_date) if law.last_modified_date else None,
            "valid_from": str(law.valid_from) if law.valid_from else None,
            "valid_until": str(law.valid_until) if law.valid_until else None,
            "full_text": law.full_text,
            "source_url": law.source_url,
            "metadata": law.metadata,
        }

    async def search_case_law(
        self,
        query: str,
        jurisdiction: Optional[List[str]] = None,
        court_level: Optional[List[str]] = None,
        limit: int = 10,
        min_score: Optional[float] = None,
    ) -> Dict[str, Any]:
        """
        Search case law.

        Args:
            query: Search query
            jurisdiction: Filter by jurisdictions
            court_level: Filter by court levels
            limit: Maximum number of results
            min_score: Minimum similarity score

        Returns:
            Dictionary with search results
        """
        results = await self.vector_store.search_case_law(
            query=query,
            limit=limit,
            min_score=min_score,
            jurisdiction_filter=jurisdiction,
            court_level_filter=court_level,
        )

        # Fetch case law details
        search_results = []
        for result in results:
            stmt = select(CaseLaw).where(CaseLaw.id == result["case_law_id"])
            case_result = await self.session.execute(stmt)
            case_law = case_result.scalar_one_or_none()

            if case_law:
                search_results.append({
                    "id": str(case_law.id),
                    "type": "case_law",
                    "court": case_law.court,
                    "case_number": case_law.case_number,
                    "decision_date": str(case_law.decision_date),
                    "title": case_law.title,
                    "content": result["content"],
                    "similarity": result["similarity"],
                    "jurisdiction": case_law.jurisdiction.value,
                    "source_url": case_law.source_url,
                })

        return {
            "query": query,
            "results": search_results,
            "total": len(search_results),
        }

    async def get_legal_changes(
        self,
        days: int = 30,
        jurisdiction: Optional[List[str]] = None,
        limit: int = 50,
    ) -> Dict[str, Any]:
        """
        Get recent legal changes.

        Args:
            days: Number of days to look back
            jurisdiction: Filter by jurisdictions
            limit: Maximum number of results

        Returns:
            Dictionary with legal changes
        """
        from datetime import datetime, timedelta

        cutoff_date = datetime.now().date() - timedelta(days=days)

        stmt = select(Law).where(
            (Law.last_modified_date >= cutoff_date) | (Law.publication_date >= cutoff_date)
        )

        if jurisdiction:
            stmt = stmt.where(Law.jurisdiction.in_(jurisdiction))

        stmt = stmt.order_by(desc(Law.last_modified_date)).limit(limit)

        result = await self.session.execute(stmt)
        laws = result.scalars().all()

        changes = []
        for law in laws:
            change_date = law.last_modified_date or law.publication_date
            change_type = "modified"

            if law.publication_date and law.publication_date >= cutoff_date:
                change_type = "new"

            if law.valid_until and law.valid_until < datetime.now().date():
                change_type = "repealed"

            changes.append({
                "id": str(law.id),
                "title": law.title,
                "abbreviation": law.abbreviation,
                "jurisdiction": law.jurisdiction.value,
                "document_type": law.document_type.value,
                "change_date": str(change_date),
                "change_type": change_type,
                "source_url": law.source_url,
            })

        return {
            "changes": changes,
            "total": len(changes),
            "days": days,
        }

    async def get_related_laws(
        self,
        law_id: UUID,
        limit: int = 5,
    ) -> Dict[str, Any]:
        """
        Get related laws.

        Args:
            law_id: Law UUID
            limit: Maximum number of results

        Returns:
            Dictionary with related laws
        """
        # Check if law exists
        stmt = select(Law).where(Law.id == law_id)
        result = await self.session.execute(stmt)
        law = result.scalar_one_or_none()

        if not law:
            return {"error": "Law not found", "law_id": str(law_id)}

        # Find similar laws
        similar_chunks = await self.vector_store.get_similar_laws(
            law_id=law_id,
            limit=limit,
        )

        # Get unique law IDs
        related_law_ids = list({chunk["law_id"] for chunk in similar_chunks})

        related_laws = []
        for related_law_id in related_law_ids[:limit]:
            stmt = select(Law).where(Law.id == related_law_id)
            result = await self.session.execute(stmt)
            related_law = result.scalar_one_or_none()

            if related_law:
                best_similarity = max(
                    chunk["similarity"]
                    for chunk in similar_chunks
                    if chunk["law_id"] == related_law_id
                )

                related_laws.append({
                    "id": str(related_law.id),
                    "title": related_law.title,
                    "abbreviation": related_law.abbreviation,
                    "jurisdiction": related_law.jurisdiction.value,
                    "document_type": related_law.document_type.value,
                    "similarity": best_similarity,
                    "source_url": related_law.source_url,
                })

        return {
            "law_id": str(law_id),
            "related_laws": related_laws,
            "total": len(related_laws),
        }
