"""Database models for legal documents - Compatible with MCP Server schema."""

from datetime import datetime, date
from typing import Optional, List
from uuid import uuid4
from sqlalchemy import (
    Column, String, Text, DateTime, Date, Float, Index, 
    Boolean, ForeignKey, Enum as SQLEnum, Integer, func
)
from sqlalchemy.dialects.postgresql import ARRAY, JSONB, UUID as PGUUID
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship
import enum

Base = declarative_base()


class JurisdictionType(str, enum.Enum):
    """Jurisdiction types - compatible with MCP Server."""
    FEDERAL = "federal"  # Bundesrecht
    EU = "eu"  # EU-Recht
    BAVARIA = "bavaria"  # Bayern
    OTHER = "other"


class DocumentType(str, enum.Enum):
    """Document type enumeration - compatible with MCP Server."""
    LAW = "law"  # Gesetz
    REGULATION = "regulation"  # Verordnung
    DIRECTIVE = "directive"  # Richtlinie (EU)
    DECISION = "decision"  # Beschluss
    OTHER = "other"


class SourceType(str, enum.Enum):
    """Source system enumeration."""
    BUNDESRECHT = "bundesrecht"
    BAYERN = "bayern"
    EURLEX = "eurlex"


class Law(Base):
    """Legal document metadata - compatible with MCP Server 'laws' table."""
    
    __tablename__ = "laws"
    
    # UUID primary key (matches MCP Server)
    id = Column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    
    # Document metadata
    title = Column(Text, nullable=False)
    abbreviation = Column(String(100), index=True)
    jurisdiction = Column(SQLEnum(JurisdictionType), nullable=False, index=True)
    document_type = Column(SQLEnum(DocumentType), nullable=False, index=True)
    
    # URLs and identifiers
    source_url = Column(Text, nullable=False, unique=True)
    official_number = Column(String(100))  # CELEX, etc.
    
    # Dates
    publication_date = Column(Date)
    last_modified_date = Column(Date)
    valid_from = Column(Date)
    valid_until = Column(Date)
    
    # Content
    full_text = Column(Text)
    
    # Additional metadata (includes source, content_hash, etc.)
    meta = Column("metadata", JSONB, nullable=False, default=dict, server_default='{}')
    
    # Timestamps
    created_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now())
    updated_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now(), onupdate=func.now())
    
    # Relationships
    chunks = relationship("LawChunk", back_populates="law", cascade="all, delete-orphan")
    
    # Indexes
    __table_args__ = (
        Index("idx_jurisdiction_type", "jurisdiction", "document_type"),
        Index("idx_publication_date", "publication_date"),
    )


class LawChunk(Base):
    """Text chunks with embeddings for semantic search - compatible with MCP Server."""
    
    __tablename__ = "law_chunks"
    
    # UUID primary key (matches MCP Server)
    id = Column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    
    # Foreign key to laws table
    law_id = Column(PGUUID(as_uuid=True), ForeignKey("laws.id", ondelete="CASCADE"), nullable=False, index=True)
    
    # Chunk information
    chunk_index = Column(Integer, nullable=False)
    section_reference = Column(String(100))  # § number, Art., etc.
    
    # Content
    content = Column(Text, nullable=False)
    
    # Embedding with pgvector (768 dimensions for German Legal BERT)
    embedding = Column(ARRAY(Float), nullable=True)
    
    # Metadata
    meta = Column("metadata", JSONB, nullable=False, default=dict, server_default='{}')
    
    # Timestamps
    created_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now())
    
    # Relationships
    law = relationship("Law", back_populates="chunks")
    
    # Indexes
    __table_args__ = (
        Index("idx_law_chunk", "law_id", "chunk_index"),
    )


class CaseLaw(Base):
    """Court decisions - compatible with MCP Server."""
    
    __tablename__ = "case_law"
    
    id = Column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    
    # Court information
    court = Column(String(200), nullable=False, index=True)
    court_level = Column(String(50), index=True)  # "bgh", "vg", etc.
    case_number = Column(String(100), nullable=False, index=True)
    
    # Metadata
    title = Column(Text)
    decision_date = Column(Date, nullable=False, index=True)
    jurisdiction = Column(SQLEnum(JurisdictionType), nullable=False, index=True)
    
    # URLs and identifiers
    source_url = Column(Text, nullable=False, unique=True)
    ecli = Column(String(200))
    
    # Content
    full_text = Column(Text)
    summary = Column(Text)
    keywords = Column(ARRAY(Text))
    
    # Metadata
    meta = Column("metadata", JSONB, nullable=False, default=dict, server_default='{}')
    
    # Timestamps
    created_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now())
    updated_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now(), onupdate=func.now())
    
    # Relationships
    chunks = relationship("CaseLawChunk", back_populates="case_law", cascade="all, delete-orphan")
    
    __table_args__ = (
        Index("idx_court_date", "court", "decision_date"),
    )


class CaseLawChunk(Base):
    """Case law chunks with embeddings."""
    
    __tablename__ = "case_law_chunks"
    
    id = Column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    case_law_id = Column(PGUUID(as_uuid=True), ForeignKey("case_law.id", ondelete="CASCADE"), nullable=False, index=True)
    
    chunk_index = Column(Integer, nullable=False)
    content = Column(Text, nullable=False)
    embedding = Column(ARRAY(Float), nullable=True)
    
    meta = Column("metadata", JSONB, nullable=False, default=dict, server_default='{}')
    created_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now())
    
    # Relationships
    case_law = relationship("CaseLaw", back_populates="chunks")


class CrawlRun(Base):
    """Log of crawler runs - compatible with MCP Server 'crawl_runs' table."""
    
    __tablename__ = "crawl_runs"
    
    id = Column(PGUUID(as_uuid=True), primary_key=True, default=uuid4)
    
    # Crawler information
    source = Column(String(100), nullable=False, index=True)
    status = Column(String(50), nullable=False, index=True)  # "running", "completed", "failed"
    
    # Statistics
    started_at = Column(DateTime(timezone=True), nullable=False, index=True)
    completed_at = Column(DateTime(timezone=True))
    documents_processed = Column(Integer, default=0)
    documents_created = Column(Integer, default=0)
    documents_updated = Column(Integer, default=0)
    errors_count = Column(Integer, default=0)
    
    # Error information
    error_details = Column(JSONB, nullable=False, default=list, server_default='[]')
    meta = Column("metadata", JSONB, nullable=False, default=dict, server_default='{}')
    
    # Timestamps
    created_at = Column(DateTime(timezone=True), nullable=False, server_default=func.now())
    
    __table_args__ = (
        Index("idx_crawl_source_status", "source", "status"),
        Index("idx_crawl_started_at", "started_at"),
    )
