"""Pydantic schemas for API request/response validation."""

from datetime import date, datetime
from typing import Any, Dict, List, Optional
from uuid import UUID

from pydantic import BaseModel, Field

from src.db.models import CourtLevel, DocumentType, JurisdictionType


# Base schemas
class LawBase(BaseModel):
    """Base schema for Law."""

    title: str
    abbreviation: Optional[str] = None
    jurisdiction: JurisdictionType
    document_type: DocumentType
    source_url: str
    official_number: Optional[str] = None
    publication_date: Optional[date] = None
    last_modified_date: Optional[date] = None
    valid_from: Optional[date] = None
    valid_until: Optional[date] = None
    full_text: Optional[str] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class LawResponse(LawBase):
    """Response schema for Law."""

    id: UUID
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class LawChunkResponse(BaseModel):
    """Response schema for Law Chunk."""

    id: UUID
    law_id: UUID
    chunk_index: int
    section_reference: Optional[str] = None
    content: str
    metadata: Dict[str, Any] = Field(default_factory=dict)
    created_at: datetime

    class Config:
        from_attributes = True


class CaseLawBase(BaseModel):
    """Base schema for Case Law."""

    court: str
    court_level: Optional[CourtLevel] = None
    case_number: str
    decision_date: date
    title: Optional[str] = None
    jurisdiction: JurisdictionType
    source_url: str
    ecli: Optional[str] = None
    full_text: Optional[str] = None
    summary: Optional[str] = None
    keywords: Optional[List[str]] = None
    referenced_laws: Optional[List[UUID]] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class CaseLawResponse(CaseLawBase):
    """Response schema for Case Law."""

    id: UUID
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


# Search request/response schemas
class SearchLawsRequest(BaseModel):
    """Request schema for law search."""

    query: str = Field(..., description="Search query text")
    jurisdiction: Optional[List[JurisdictionType]] = Field(
        None, description="Filter by jurisdictions"
    )
    document_type: Optional[List[DocumentType]] = Field(
        None, description="Filter by document types"
    )
    include_case_law: bool = Field(False, description="Include case law in results")
    limit: int = Field(10, ge=1, le=100, description="Maximum number of results")
    min_score: Optional[float] = Field(
        None, ge=0.0, le=1.0, description="Minimum similarity score"
    )


class SearchResultItem(BaseModel):
    """Individual search result item."""

    id: UUID
    type: str = Field(..., description="Result type: 'law' or 'case_law'")
    title: str
    content: str = Field(..., description="Relevant text snippet")
    similarity: float = Field(..., description="Similarity score (0-1)")
    jurisdiction: JurisdictionType
    metadata: Dict[str, Any] = Field(default_factory=dict)


class SearchLawsResponse(BaseModel):
    """Response schema for law search."""

    query: str
    results: List[SearchResultItem]
    total: int
    execution_time_ms: float


class GetLawRequest(BaseModel):
    """Request schema for getting a law by ID or abbreviation."""

    id: Optional[UUID] = Field(None, description="Law UUID")
    abbreviation: Optional[str] = Field(None, description="Law abbreviation")


class SearchCaseLawRequest(BaseModel):
    """Request schema for case law search."""

    query: str = Field(..., description="Search query text")
    jurisdiction: Optional[List[JurisdictionType]] = Field(
        None, description="Filter by jurisdictions"
    )
    court_level: Optional[List[CourtLevel]] = Field(None, description="Filter by court levels")
    decision_date_from: Optional[date] = Field(None, description="Filter by decision date (from)")
    decision_date_to: Optional[date] = Field(None, description="Filter by decision date (to)")
    limit: int = Field(10, ge=1, le=100, description="Maximum number of results")
    min_score: Optional[float] = Field(
        None, ge=0.0, le=1.0, description="Minimum similarity score"
    )


class LegalChangesRequest(BaseModel):
    """Request schema for getting recent legal changes."""

    days: int = Field(30, ge=1, le=365, description="Number of days to look back")
    jurisdiction: Optional[List[JurisdictionType]] = Field(
        None, description="Filter by jurisdictions"
    )
    limit: int = Field(50, ge=1, le=200, description="Maximum number of results")


class LegalChangeItem(BaseModel):
    """Individual legal change item."""

    id: UUID
    title: str
    abbreviation: Optional[str] = None
    jurisdiction: JurisdictionType
    document_type: DocumentType
    change_date: date
    change_type: str = Field(..., description="Type of change: 'new', 'modified', 'repealed'")
    source_url: str


class LegalChangesResponse(BaseModel):
    """Response schema for legal changes."""

    changes: List[LegalChangeItem]
    total: int
    days: int


class RelatedLawsRequest(BaseModel):
    """Request schema for getting related laws."""

    law_id: UUID = Field(..., description="UUID of the law")
    limit: int = Field(5, ge=1, le=20, description="Maximum number of results")


class RelatedLawItem(BaseModel):
    """Individual related law item."""

    id: UUID
    title: str
    abbreviation: Optional[str] = None
    jurisdiction: JurisdictionType
    document_type: DocumentType
    similarity: float = Field(..., description="Similarity score (0-1)")
    relation_type: Optional[str] = Field(None, description="Type of relation if explicitly defined")


class RelatedLawsResponse(BaseModel):
    """Response schema for related laws."""

    law_id: UUID
    related_laws: List[RelatedLawItem]
    total: int


# Error response schema
class ErrorResponse(BaseModel):
    """Error response schema."""

    error: str = Field(..., description="Error message")
    detail: Optional[str] = Field(None, description="Detailed error information")
    status_code: int = Field(..., description="HTTP status code")


# Health check
class HealthResponse(BaseModel):
    """Health check response."""

    status: str = Field(..., description="Service status")
    version: str = Field(..., description="API version")
    database: str = Field(..., description="Database status")
    model: str = Field(..., description="ML model status")
