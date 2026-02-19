"""Database models for Legal MCP Server using SQLAlchemy 2.0."""

import enum
from datetime import date, datetime
from typing import List, Optional
from uuid import UUID, uuid4

from sqlalchemy import (
    ARRAY,
    Boolean,
    Date,
    DateTime,
    Float,
    ForeignKey,
    Integer,
    String,
    Text,
    func,
)
from sqlalchemy.dialects.postgresql import JSONB, UUID as PGUUID
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship


class Base(DeclarativeBase):
    """Base class for all database models."""

    pass


# Enums matching PostgreSQL types - use same case as database
class JurisdictionType(str, enum.Enum):
    """Jurisdiction types for legal documents."""

    federal = "federal"
    eu = "eu"
    bavaria = "bavaria"
    other = "other"


class DocumentType(str, enum.Enum):
    """Document types for legal texts."""

    law = "law"
    regulation = "regulation"
    directive = "directive"
    decision = "decision"
    other = "other"


class CourtLevel(str, enum.Enum):
    """Court hierarchy levels."""

    bgh = "bgh"  # Bundesgerichtshof
    bverwg = "bverwg"  # Bundesverwaltungsgericht
    bfh = "bfh"  # Bundesfinanzhof
    bsg = "bsg"  # Bundessozialgericht
    bag = "bag"  # Bundesarbeitsgericht
    lg = "lg"  # Landgericht
    ag = "ag"  # Amtsgericht
    vg = "vg"  # Verwaltungsgericht
    other = "other"


class Law(Base):
    """Main legislation documents."""

    __tablename__ = "laws"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    title: Mapped[str] = mapped_column(Text, nullable=False)
    abbreviation: Mapped[Optional[str]] = mapped_column(String(100), nullable=True, index=True)
    jurisdiction: Mapped[JurisdictionType] = mapped_column(nullable=False, index=True)
    document_type: Mapped[DocumentType] = mapped_column(nullable=False, index=True)
    source_url: Mapped[str] = mapped_column(Text, nullable=False, unique=True)
    official_number: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    publication_date: Mapped[Optional[date]] = mapped_column(Date, nullable=True, index=True)
    last_modified_date: Mapped[Optional[date]] = mapped_column(Date, nullable=True)
    valid_from: Mapped[Optional[date]] = mapped_column(Date, nullable=True)
    valid_until: Mapped[Optional[date]] = mapped_column(Date, nullable=True)
    full_text: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    meta: Mapped[dict] = mapped_column("metadata", JSONB, nullable=False, default=dict, server_default="{}")
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now(), onupdate=func.now()
    )

    # Relationships
    chunks: Mapped[List["LawChunk"]] = relationship(
        "LawChunk", back_populates="law", cascade="all, delete-orphan"
    )


class LawChunk(Base):
    """Chunked law texts with embeddings for semantic search."""

    __tablename__ = "law_chunks"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    law_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("laws.id", ondelete="CASCADE"), nullable=False, index=True
    )
    chunk_index: Mapped[int] = mapped_column(Integer, nullable=False)
    section_reference: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    content: Mapped[str] = mapped_column(Text, nullable=False)
    embedding: Mapped[Optional[List[float]]] = mapped_column(ARRAY(Float), nullable=True)
    meta: Mapped[dict] = mapped_column("metadata", JSONB, nullable=False, default=dict, server_default="{}")
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )

    # Relationships
    law: Mapped["Law"] = relationship("Law", back_populates="chunks")


class CaseLaw(Base):
    """Court decisions and case law."""

    __tablename__ = "case_law"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    court: Mapped[str] = mapped_column(String(200), nullable=False, index=True)
    court_level: Mapped[Optional[CourtLevel]] = mapped_column(nullable=True, index=True)
    case_number: Mapped[str] = mapped_column(String(100), nullable=False, index=True)
    decision_date: Mapped[date] = mapped_column(Date, nullable=False, index=True)
    title: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    jurisdiction: Mapped[JurisdictionType] = mapped_column(nullable=False, index=True)
    source_url: Mapped[str] = mapped_column(Text, nullable=False, unique=True)
    ecli: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    full_text: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    summary: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    keywords: Mapped[Optional[List[str]]] = mapped_column(ARRAY(Text), nullable=True)
    referenced_laws: Mapped[Optional[List[UUID]]] = mapped_column(
        ARRAY(PGUUID(as_uuid=True)), nullable=True
    )
    meta: Mapped[dict] = mapped_column("metadata", JSONB, nullable=False, default=dict, server_default="{}")
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now(), onupdate=func.now()
    )

    # Relationships
    chunks: Mapped[List["CaseLawChunk"]] = relationship(
        "CaseLawChunk", back_populates="case_law", cascade="all, delete-orphan"
    )


class CaseLawChunk(Base):
    """Chunked case law texts with embeddings for semantic search."""

    __tablename__ = "case_law_chunks"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    case_law_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("case_law.id", ondelete="CASCADE"), nullable=False, index=True
    )
    chunk_index: Mapped[int] = mapped_column(Integer, nullable=False)
    content: Mapped[str] = mapped_column(Text, nullable=False)
    embedding: Mapped[Optional[List[float]]] = mapped_column(ARRAY(Float), nullable=True)
    meta: Mapped[dict] = mapped_column("metadata", JSONB, nullable=False, default=dict, server_default="{}")
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )

    # Relationships
    case_law: Mapped["CaseLaw"] = relationship("CaseLaw", back_populates="chunks")


class CrawlRun(Base):
    """Tracking table for crawler executions."""

    __tablename__ = "crawl_runs"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    source: Mapped[str] = mapped_column(String(100), nullable=False, index=True)
    status: Mapped[str] = mapped_column(String(50), nullable=False, index=True)
    started_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, index=True
    )
    completed_at: Mapped[Optional[datetime]] = mapped_column(DateTime(timezone=True), nullable=True)
    documents_processed: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    documents_created: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    documents_updated: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    errors_count: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    error_details: Mapped[list] = mapped_column(JSONB, nullable=False, default=list, server_default="[]")
    meta: Mapped[dict] = mapped_column("metadata", JSONB, nullable=False, default=dict, server_default="{}")
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )


class APIKey(Base):
    """API keys for authentication."""

    __tablename__ = "api_keys"

    id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    key_hash: Mapped[str] = mapped_column(String(256), nullable=False, unique=True, index=True)
    name: Mapped[str] = mapped_column(String(200), nullable=False)
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    is_active: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True, index=True)
    rate_limit: Mapped[int] = mapped_column(Integer, nullable=False, default=1000)
    allowed_origins: Mapped[Optional[List[str]]] = mapped_column(ARRAY(Text), nullable=True)
    last_used_at: Mapped[Optional[datetime]] = mapped_column(DateTime(timezone=True), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    expires_at: Mapped[Optional[datetime]] = mapped_column(DateTime(timezone=True), nullable=True)


class RelatedLaw(Base):
    """Junction table for related laws."""

    __tablename__ = "related_laws"

    law_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), nullable=False, primary_key=True
    )
    related_law_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), nullable=False, primary_key=True
    )
    relation_type: Mapped[str] = mapped_column(String(50), nullable=False)
    confidence_score: Mapped[Optional[float]] = mapped_column(Float, nullable=True)
