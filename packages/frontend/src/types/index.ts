// ─── Search ──────────────────────────────────────────────────────────────────

export interface SearchResult {
  chunk_id: string;
  document_id: string;
  title: string;
  url: string;
  jurisdiction: string;
  doc_type: DocType;
  language: string;
  snippet: string;
  score: number;
  published_at: string | null;
  source_name: string;
}

export type DocType = 'statute' | 'regulation' | 'case' | 'directive';

export interface SearchParams {
  q: string;
  jurisdiction?: string;
  doc_type?: DocType;
  language?: string;
  limit?: number;
  offset?: number;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
  limit: number;
  offset: number;
  query_embedding_ms?: number;
  search_ms?: number;
}

// ─── Documents ───────────────────────────────────────────────────────────────

export interface Document {
  id: string;
  external_id: string;
  title: string;
  url: string;
  content_md: string;
  summary: string | null;
  doc_type: DocType;
  jurisdiction: string;
  language: string;
  published_at: string | null;
  source_id: string;
  source_name?: string;
}

export interface DocumentVersion {
  id: string;
  document_id: string;
  version_number: number;
  content_hash: string;
  changed_at: string;
  change_summary: string | null;
}

// ─── Sources ─────────────────────────────────────────────────────────────────

export interface Source {
  id: string;
  name: string;
  jurisdiction: string;
  language: string;
  base_url: string;
  crawler_type: string;
  cron_schedule: string;
  enabled: boolean;
}

// ─── Changes ─────────────────────────────────────────────────────────────────

export interface ChangesParams {
  since?: string;
  jurisdiction?: string;
  limit?: number;
  offset?: number;
}

export interface ChangesResponse {
  results: DocumentVersion[];
  total: number;
  limit: number;
  offset: number;
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/** RFC 7807 Problem Details */
export interface ApiError {
  type?: string;
  title?: string;
  status: number;
  detail?: string;
  /** Legacy compat fields */
  error?: string;
  details?: string;
}

// ─── Chat (deferred — no new endpoint) ───────────────────────────────────────

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp?: string;
}

export interface ChatSource {
  title: string;
  jurisdiction: string;
  content: string;
  metadata?: {
    abbreviation?: string;
    section_reference?: string;
    source_url?: string;
    document_type?: string;
  };
}

export interface ChatRequest {
  message: string;
  history?: ChatMessage[];
  context?: ChatSource[];
}

export interface ChatResponse {
  answer: string;
  sources: ChatSource[];
  model_used?: string;
  context_size?: number;
}
