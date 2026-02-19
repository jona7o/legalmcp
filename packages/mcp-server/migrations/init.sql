-- Legal MCP Database Schema
-- PostgreSQL 15+ with pgvector extension

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Jurisdictions enum
CREATE TYPE jurisdiction_type AS ENUM ('federal', 'eu', 'bavaria', 'other');

-- Document types enum
CREATE TYPE document_type AS ENUM ('law', 'regulation', 'directive', 'decision', 'other');

-- Court levels enum
CREATE TYPE court_level AS ENUM ('bgh', 'bverwg', 'bfh', 'bsg', 'bag', 'lg', 'ag', 'vg', 'other');

-- Laws table (main legislation documents)
CREATE TABLE laws (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title TEXT NOT NULL,
    abbreviation VARCHAR(100),
    jurisdiction jurisdiction_type NOT NULL,
    document_type document_type NOT NULL,
    source_url TEXT NOT NULL UNIQUE,
    official_number VARCHAR(100),
    publication_date DATE,
    last_modified_date DATE,
    valid_from DATE,
    valid_until DATE,
    full_text TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Law chunks table (for semantic search)
CREATE TABLE law_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    law_id UUID NOT NULL REFERENCES laws(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    section_reference VARCHAR(100),
    content TEXT NOT NULL,
    embedding vector(768),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(law_id, chunk_index)
);

-- Case law table (court decisions)
CREATE TABLE case_law (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    court VARCHAR(200) NOT NULL,
    court_level court_level,
    case_number VARCHAR(100) NOT NULL,
    decision_date DATE NOT NULL,
    title TEXT,
    jurisdiction jurisdiction_type NOT NULL,
    source_url TEXT NOT NULL UNIQUE,
    ecli VARCHAR(200),
    full_text TEXT,
    summary TEXT,
    keywords TEXT[],
    referenced_laws UUID[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Case law chunks table (for semantic search)
CREATE TABLE case_law_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    case_law_id UUID NOT NULL REFERENCES case_law(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(768),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(case_law_id, chunk_index)
);

-- Crawl runs table (tracking crawler execution)
CREATE TABLE crawl_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    documents_processed INTEGER DEFAULT 0,
    documents_created INTEGER DEFAULT 0,
    documents_updated INTEGER DEFAULT 0,
    errors_count INTEGER DEFAULT 0,
    error_details JSONB DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- API Keys table (authentication)
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash VARCHAR(256) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    rate_limit INTEGER DEFAULT 1000,
    allowed_origins TEXT[],
    last_used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE
);

-- Related laws junction table
CREATE TABLE related_laws (
    law_id UUID NOT NULL REFERENCES laws(id) ON DELETE CASCADE,
    related_law_id UUID NOT NULL REFERENCES laws(id) ON DELETE CASCADE,
    relation_type VARCHAR(50) NOT NULL,
    confidence_score FLOAT,
    PRIMARY KEY (law_id, related_law_id)
);

-- Indexes for performance

-- Laws indexes
CREATE INDEX idx_laws_abbreviation ON laws(abbreviation);
CREATE INDEX idx_laws_jurisdiction ON laws(jurisdiction);
CREATE INDEX idx_laws_document_type ON laws(document_type);
CREATE INDEX idx_laws_publication_date ON laws(publication_date);
CREATE INDEX idx_laws_title_trgm ON laws USING gin(title gin_trgm_ops);
CREATE INDEX idx_laws_metadata ON laws USING gin(metadata);

-- Law chunks indexes
CREATE INDEX idx_law_chunks_law_id ON law_chunks(law_id);
CREATE INDEX idx_law_chunks_embedding ON law_chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Case law indexes
CREATE INDEX idx_case_law_court ON case_law(court);
CREATE INDEX idx_case_law_court_level ON case_law(court_level);
CREATE INDEX idx_case_law_case_number ON case_law(case_number);
CREATE INDEX idx_case_law_decision_date ON case_law(decision_date);
CREATE INDEX idx_case_law_jurisdiction ON case_law(jurisdiction);
CREATE INDEX idx_case_law_keywords ON case_law USING gin(keywords);
CREATE INDEX idx_case_law_referenced_laws ON case_law USING gin(referenced_laws);
CREATE INDEX idx_case_law_metadata ON case_law USING gin(metadata);

-- Case law chunks indexes
CREATE INDEX idx_case_law_chunks_case_law_id ON case_law_chunks(case_law_id);
CREATE INDEX idx_case_law_chunks_embedding ON case_law_chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Crawl runs indexes
CREATE INDEX idx_crawl_runs_source ON crawl_runs(source);
CREATE INDEX idx_crawl_runs_status ON crawl_runs(status);
CREATE INDEX idx_crawl_runs_started_at ON crawl_runs(started_at DESC);

-- API Keys indexes
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_is_active ON api_keys(is_active);

-- Create trigram extension for fuzzy text search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_laws_updated_at BEFORE UPDATE ON laws
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_case_law_updated_at BEFORE UPDATE ON case_law
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default API key for development (key: dev_key_12345)
INSERT INTO api_keys (key_hash, name, description, is_active, rate_limit)
VALUES (
    '5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8', -- SHA256 of 'dev_key_12345'
    'Development Key',
    'Default API key for local development',
    TRUE,
    10000
);

-- Grant permissions (adjust user as needed)
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO legal;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO legal;
