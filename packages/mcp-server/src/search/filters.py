"""Search filters and utilities."""

from datetime import date, datetime
from typing import List, Optional

from sqlalchemy import Select, and_, or_

from src.db.models import CaseLaw, CourtLevel, DocumentType, JurisdictionType, Law


class SearchFilters:
    """Utility class for building search filters."""

    @staticmethod
    def apply_law_filters(
        stmt: Select,
        jurisdiction: Optional[List[JurisdictionType]] = None,
        document_type: Optional[List[DocumentType]] = None,
        publication_date_from: Optional[date] = None,
        publication_date_to: Optional[date] = None,
        abbreviation: Optional[str] = None,
        title_keywords: Optional[List[str]] = None,
    ) -> Select:
        """
        Apply filters to law query.

        Args:
            stmt: SQLAlchemy select statement
            jurisdiction: Filter by jurisdictions
            document_type: Filter by document types
            publication_date_from: Filter by publication date (from)
            publication_date_to: Filter by publication date (to)
            abbreviation: Filter by abbreviation (exact match)
            title_keywords: Filter by title keywords (case-insensitive)

        Returns:
            Modified select statement
        """
        conditions = []

        if jurisdiction:
            conditions.append(Law.jurisdiction.in_(jurisdiction))

        if document_type:
            conditions.append(Law.document_type.in_(document_type))

        if publication_date_from:
            conditions.append(Law.publication_date >= publication_date_from)

        if publication_date_to:
            conditions.append(Law.publication_date <= publication_date_to)

        if abbreviation:
            conditions.append(Law.abbreviation.ilike(f"%{abbreviation}%"))

        if title_keywords:
            # Search for any of the keywords in the title
            keyword_conditions = [
                Law.title.ilike(f"%{keyword}%") for keyword in title_keywords
            ]
            conditions.append(or_(*keyword_conditions))

        if conditions:
            stmt = stmt.where(and_(*conditions))

        return stmt

    @staticmethod
    def apply_case_law_filters(
        stmt: Select,
        jurisdiction: Optional[List[JurisdictionType]] = None,
        court_level: Optional[List[CourtLevel]] = None,
        decision_date_from: Optional[date] = None,
        decision_date_to: Optional[date] = None,
        court: Optional[str] = None,
        keywords: Optional[List[str]] = None,
    ) -> Select:
        """
        Apply filters to case law query.

        Args:
            stmt: SQLAlchemy select statement
            jurisdiction: Filter by jurisdictions
            court_level: Filter by court levels
            decision_date_from: Filter by decision date (from)
            decision_date_to: Filter by decision date (to)
            court: Filter by court name
            keywords: Filter by keywords

        Returns:
            Modified select statement
        """
        conditions = []

        if jurisdiction:
            conditions.append(CaseLaw.jurisdiction.in_(jurisdiction))

        if court_level:
            conditions.append(CaseLaw.court_level.in_(court_level))

        if decision_date_from:
            conditions.append(CaseLaw.decision_date >= decision_date_from)

        if decision_date_to:
            conditions.append(CaseLaw.decision_date <= decision_date_to)

        if court:
            conditions.append(CaseLaw.court.ilike(f"%{court}%"))

        if keywords:
            # Check if any of the provided keywords are in the keywords array
            conditions.append(CaseLaw.keywords.overlap(keywords))

        if conditions:
            stmt = stmt.where(and_(*conditions))

        return stmt


def parse_date(date_str: Optional[str]) -> Optional[date]:
    """
    Parse date string to date object.

    Args:
        date_str: Date string in ISO format (YYYY-MM-DD)

    Returns:
        Date object or None
    """
    if not date_str:
        return None

    try:
        return datetime.fromisoformat(date_str).date()
    except ValueError:
        return None
