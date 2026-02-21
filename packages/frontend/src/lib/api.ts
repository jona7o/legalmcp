import type {
  SearchParams,
  SearchResponse,
  Document,
  DocumentVersion,
  Source,
  ChangesParams,
  ChangesResponse,
  ApiError,
  ChatRequest,
  ChatResponse,
} from '@/types';

const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

// ─── RFC 7807 error parsing ───────────────────────────────────────────────────

async function parseErrorResponse(response: Response): Promise<ApiError> {
  const base: ApiError = { status: response.status };
  try {
    const body = await response.json();
    return {
      ...base,
      type: body.type,
      title: body.title,
      detail: body.detail,
      // legacy compat
      error: body.title || response.statusText,
      details: body.detail,
    };
  } catch {
    return { ...base, error: response.statusText };
  }
}

// ─── Core fetch wrapper ───────────────────────────────────────────────────────

async function apiFetch<T>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const url = `${API_BASE_URL}${path}`;
  let response: Response;

  try {
    response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });
  } catch {
    const err: ApiError = {
      status: 0,
      error: 'Network error',
      details: 'Could not connect to the server.',
    };
    throw err;
  }

  if (!response.ok) {
    throw await parseErrorResponse(response);
  }

  return response.json() as Promise<T>;
}

// ─── Search ───────────────────────────────────────────────────────────────────

export async function searchLaws(params: SearchParams): Promise<SearchResponse> {
  const query = new URLSearchParams({ q: params.q });

  if (params.jurisdiction) query.set('jurisdiction', params.jurisdiction);
  if (params.doc_type) query.set('doc_type', params.doc_type);
  if (params.language) query.set('language', params.language);
  if (params.limit != null) query.set('limit', String(params.limit));
  if (params.offset != null) query.set('offset', String(params.offset));

  return apiFetch<SearchResponse>(`/api/v1/search?${query.toString()}`);
}

// ─── Documents ────────────────────────────────────────────────────────────────

export async function getDocument(id: string): Promise<Document> {
  return apiFetch<Document>(`/api/v1/documents/${encodeURIComponent(id)}`);
}

export async function getDocumentVersions(id: string): Promise<DocumentVersion[]> {
  return apiFetch<DocumentVersion[]>(
    `/api/v1/documents/${encodeURIComponent(id)}/versions`
  );
}

// ─── Sources ─────────────────────────────────────────────────────────────────

export async function getSources(jurisdiction?: string): Promise<Source[]> {
  const query = jurisdiction
    ? `?jurisdiction=${encodeURIComponent(jurisdiction)}`
    : '';
  return apiFetch<Source[]>(`/api/v1/sources${query}`);
}

// ─── Changes ─────────────────────────────────────────────────────────────────

export async function getChanges(params: ChangesParams): Promise<ChangesResponse> {
  const query = new URLSearchParams();

  if (params.since) query.set('since', params.since);
  if (params.jurisdiction) query.set('jurisdiction', params.jurisdiction);
  if (params.limit != null) query.set('limit', String(params.limit));
  if (params.offset != null) query.set('offset', String(params.offset));

  const qs = query.toString();
  return apiFetch<ChangesResponse>(`/api/v1/changes${qs ? `?${qs}` : ''}`);
}

// ─── Legacy export kept for chat page (endpoint deferred) ────────────────────

export const apiClient = {
  /** @deprecated use named functions instead */
  async chat(request: ChatRequest): Promise<ChatResponse> {
    return apiFetch<ChatResponse>('/api/chat', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },
};
