import type { 
  SearchResult, 
  SearchParams, 
  LawDocument, 
  CaseDocument, 
  Change,
  ChatRequest,
  ChatResponse,
  ApiError 
} from '@/types';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    
    try {
      const response = await fetch(url, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
        },
      });

      if (!response.ok) {
        const error: ApiError = {
          error: response.statusText,
          status: response.status,
        };
        
        try {
          const errorData = await response.json();
          error.details = errorData.detail || errorData.message;
        } catch {
          // Ignore JSON parse errors
        }
        
        throw error;
      }

      return await response.json();
    } catch (error) {
      if ((error as ApiError).status) {
        throw error;
      }
      
      throw {
        error: 'Netzwerkfehler',
        details: 'Die Verbindung zum Server konnte nicht hergestellt werden.',
        status: 0,
      } as ApiError;
    }
  }

  async search(params: SearchParams): Promise<SearchResult[]> {
    // Backend erwartet POST /api/search mit JSON body
    const requestBody = {
      query: params.query,
      limit: params.limit || 10,
      min_score: 0.7,
    };
    
    // Optional parameters
    if (params.jurisdictions?.length) {
      Object.assign(requestBody, { jurisdiction: params.jurisdictions });
    }
    
    if (params.includeCases !== undefined) {
      Object.assign(requestBody, { include_case_law: params.includeCases });
    }
    
    if (params.documentType) {
      Object.assign(requestBody, { document_type: [params.documentType] });
    }

    return this.request<SearchResult[]>(`/api/search`, {
      method: 'POST',
      body: JSON.stringify(requestBody),
    });
  }

  async getLaw(id: string): Promise<LawDocument> {
    return this.request<LawDocument>(`/api/laws/${id}`);
  }

  async getCase(id: string): Promise<CaseDocument> {
    return this.request<CaseDocument>(`/api/case-law/${id}`);
  }

  async getChanges(params?: { 
    jurisdiction?: string; 
    since?: string;
    limit?: number;
  }): Promise<Change[]> {
    const queryParams = new URLSearchParams();
    
    if (params?.jurisdiction) {
      queryParams.append('jurisdiction', params.jurisdiction);
    }
    
    if (params?.since) {
      queryParams.append('since_days', params.since);
    }
    
    if (params?.limit) {
      queryParams.append('limit', String(params.limit));
    }

    const query = queryParams.toString();
    return this.request<Change[]>(`/api/changes${query ? `?${query}` : ''}`);
  }

  async chat(request: ChatRequest): Promise<ChatResponse> {
    return this.request<ChatResponse>(`/api/chat`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }
}

export const apiClient = new ApiClient();
