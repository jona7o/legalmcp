# Legal MCP — Concept & Idea

## Vision

An AI-powered semantic search platform for German and European Union law, built on the Model Context Protocol (MCP). The goal is to make legal texts truly accessible — not just searchable by keyword, but queryable by meaning — for lawyers, researchers, students, and AI assistants alike.

## Problem

- Legal texts are scattered across multiple official government portals (Gesetze-im-Internet, Bayern.Recht, EUR-Lex), each with different formats (XML, HTML, SPARQL endpoints).
- Traditional keyword search fails for legal research: users need to find laws *by concept*, not by exact wording.
- LLMs hallucinate legal citations. There is no reliable way for AI assistants to ground their answers in actual, current legal text.
- No open, standardized protocol exists for LLMs to query legal databases as tools.

## Solution

### 1. Crawl & Ingest

Automated crawlers pull legal documents from three official sources:

- **Bundesrecht** — German federal law (Gesetze-im-Internet.de, XML)
- **Bayern.Recht** — Bavarian state law (gesetze-bayern.de, HTML)
- **EUR-Lex** — EU legislation (SPARQL + CELLAR endpoint)

Documents are parsed, normalized, chunked (with configurable size/overlap), and stored in PostgreSQL.

### 2. Semantic Embeddings

Each text chunk is embedded into a 768-dimensional vector space using Google Vertex AI (`text-embedding-004`). These embeddings are stored via the **pgvector** extension, enabling cosine-similarity search at database level.

### 3. Search & Retrieval

- **Semantic search**: Encode a natural-language query into the same vector space and find the most similar legal text chunks.
- **Filtering**: Narrow results by jurisdiction (federal, state, EU) and document type (law, regulation, directive, etc.).
- **Related laws**: Discover semantically related legislation.
- **Legal changes**: Track new, modified, and repealed legal texts over time.

### 4. RAG-Powered Legal Chat

A chat assistant powered by Gemini 2.5 Pro that:
- Automatically searches for the top relevant legal text chunks based on the user's question.
- Sends those chunks as grounded context to the LLM.
- Strictly constrains the model to only cite from provided legal texts — never from its own training data.
- Returns answers with source citations users can verify.

### 5. MCP Integration

Expose search capabilities via the **Model Context Protocol** so any MCP-compatible LLM client (e.g., Claude Desktop) can use legal search as a tool during reasoning. This turns the legal database into a first-class tool for AI assistants.

**MCP Tools:**
- `search_laws` — Semantic search across laws
- `get_law_by_id` — Retrieve a specific law
- `search_case_law` — Semantic search across court decisions
- `get_legal_changes` — Recent legal changes
- `get_related_laws` — Find related legislation

**MCP Resources:**
- `legal://jurisdictions` — Available jurisdictions
- `legal://document-types` — Available document types

## Architecture

```
  Official Legal Sources              PostgreSQL + pgvector
  ┌──────────────────┐               ┌─────────────────────┐
  │ Gesetze-im-Internet │             │ laws                │
  │ Bayern.Recht       │──▶ Crawler ──▶│ law_chunks (vec768) │
  │ EUR-Lex            │   pipeline   │ case_law            │
  └──────────────────┘               │ case_law_chunks     │
                                      └─────────┬───────────┘
                                                │
                                  ┌─────────────┴─────────────┐
                                  ▼                           ▼
                          REST API (FastAPI)          MCP Server (stdio)
                          ┌───────────────┐          ┌────────────────┐
                          │ /api/search   │          │ search_laws    │
                          │ /api/chat     │          │ get_law_by_id  │
                          │ /api/laws     │          │ search_case_law│
                          │ /api/changes  │          │ ...            │
                          └───────┬───────┘          └───────┬────────┘
                                  ▼                          ▼
                          Next.js Frontend           LLM Clients
                          (Search + Chat UI)         (Claude, etc.)
```

## Tech Stack

| Layer | Technology |
|---|---|
| Crawler | Python, aiohttp, BeautifulSoup, lxml |
| Backend API | Python, FastAPI, SQLAlchemy 2.0 (async) |
| Database | PostgreSQL 15, pgvector, pg_trgm |
| Embeddings | Google Vertex AI `text-embedding-004` |
| LLM (Chat) | Google Vertex AI Gemini 2.5 Pro |
| MCP | `mcp` Python SDK, stdio transport |
| Frontend | Next.js 14, React 18, TypeScript, Tailwind, shadcn/ui |
| Infra | Docker Compose (dev), Cloud Run + Firebase Hosting (prod) |

## Monorepo Structure

```
packages/
  crawler/       — Legal document crawlers & processing pipeline
  mcp-server/    — FastAPI REST API + MCP protocol server
  frontend/      — Next.js web application (search + chat)
  postgres/      — Custom PostgreSQL image with pgvector
```

## Key Design Decisions

- **pgvector over dedicated vector DBs**: Keeps the stack simple — one database for both relational data and vector search. Good enough for the scale of legal corpora.
- **Chunking strategy**: Legal texts are split into chunks with configurable size and overlap to balance retrieval precision vs. context completeness.
- **Strict RAG grounding**: The chat system prompt explicitly forbids the LLM from using its own knowledge, reducing hallucinated citations.
- **MCP as first-class interface**: Not just a REST API — the MCP protocol integration makes this a composable tool for any AI workflow.
- **Rate limiting & robots.txt compliance**: Crawlers respect official sources with rate limiting, retries, and robots.txt checking.

## Future Directions (to refine)

- Expand to additional German state laws (currently only Bavaria)
- Add case law from court databases (Bundesgerichtshof, BVerfG, etc.)
- Support for cross-referencing between laws (citation graph)
- User accounts and saved searches
- Webhook notifications for changes in tracked laws
- HTTP-based MCP transport (currently stdio only)
- Fine-tuned embedding models for German legal language
- Multi-language support (German/English) for EU legislation

## Legal Note

Official legal texts are in the public domain under German copyright law (Section 5(1) UrhG). This tool is not a substitute for professional legal advice.

---

*This document is a living concept note. It will be refined as the project evolves.*
