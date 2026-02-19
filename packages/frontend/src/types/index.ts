export interface SearchResult {
  id: string;
  type: 'law' | 'case';
  title: string;
  snippet: string;
  jurisdiction: 'BY' | 'DE' | 'EU';
  score: number;
  metadata?: {
    abbreviation?: string;
    date?: string;
    court?: string;
    fileNumber?: string;
  };
}

export interface SearchParams {
  query: string;
  jurisdictions?: string[];
  includeCases?: boolean;
  documentType?: string;
  limit?: number;
  offset?: number;
}

export interface LawDocument {
  id: string;
  title: string;
  abbreviation: string;
  jurisdiction: 'BY' | 'DE' | 'EU';
  content: string;
  sections: Section[];
  metadata: {
    promulgationDate?: string;
    lastModified?: string;
    citation?: string;
    url?: string;
  };
}

export interface Section {
  id: string;
  number: string;
  title: string;
  content: string;
  subsections?: Section[];
}

export interface CaseDocument {
  id: string;
  title: string;
  court: string;
  fileNumber: string;
  date: string;
  jurisdiction: 'BY' | 'DE' | 'EU';
  summary: string;
  fullText: string;
  headnotes?: string[];
  references?: Reference[];
  metadata: {
    ecli?: string;
    citation?: string;
    url?: string;
  };
}

export interface Reference {
  type: 'law' | 'case';
  id: string;
  citation: string;
  context?: string;
}

export interface Change {
  id: string;
  lawId: string;
  lawTitle: string;
  changeDate: string;
  description: string;
  affectedSections: string[];
  jurisdiction: 'BY' | 'DE' | 'EU';
}

export interface ApiError {
  error: string;
  details?: string;
  status: number;
}

// Chat interface types
export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp?: string;
}

export interface ChatSource {
  title: string;
  jurisdiction: 'BY' | 'DE' | 'EU';
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
