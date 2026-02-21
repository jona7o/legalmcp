---
title: "Global Legal Crawler & MCP RAG System"
status: completed
version: "1.2"
---

# Product Requirements Document

## Validation Checklist

### CRITICAL GATES (Must Pass)

- [x] All required sections are complete
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Problem statement is specific and measurable
- [x] Every feature has testable acceptance criteria (Gherkin format)
- [x] No contradictions between sections

### QUALITY CHECKS (Should Pass)

- [x] Problem is validated by evidence (not assumptions)
- [x] Context → Problem → Solution flow makes sense
- [x] Every persona has at least one user journey
- [x] All MoSCoW categories addressed (Must/Should/Could/Won't)
- [x] Every metric has corresponding tracking events
- [x] No feature redundancy (check for duplicates)
- [x] No technical implementation details included
- [x] A new team member could understand this PRD

---

## Product Overview

### Vision

A global, open-source platform that makes any legal and regulatory text worldwide discoverable through semantic search, accessible as grounded context for AI assistants via the Model Context Protocol, and manageable through a dynamic source administration interface — replacing keyword search with meaning-based retrieval across jurisdictions, languages, and compliance domains.

### Problem Statement

Legal texts are scattered across hundreds of government portals worldwide, each with different formats (XML, HTML, PDF, SPARQL endpoints), languages, and structures. Today's reality:

- **Fragmented access**: A lawyer researching cross-border compliance must navigate dozens of separate portals with different search interfaces. There is no single platform for multi-jurisdictional legal search. Cross-jurisdictional research that should take 1 hour takes 4-8 hours due to portal-hopping, language barriers, and manual comparison.
- **Keyword search fails for legal research**: Legal concepts are expressed differently across statutes. Searching for "employee data protection" won't find "Beschäftigtendatenschutz" (BDSG §26) or "traitement des données des salariés" (French CNIL guidelines). Users need semantic, concept-based search.
- **AI assistants hallucinate legal citations**: LLMs frequently fabricate statute numbers, article references, and case citations. Without grounded access to actual legal text, AI-assisted legal research is unreliable and potentially dangerous — a wrong citation carries professional liability.
- **Static, hardcoded systems**: The current platform supports only 3 German/EU sources via hardcoded crawlers. Adding a new jurisdiction requires writing new code, building a new parser, and deploying. This doesn't scale to global coverage.
- **No document lifecycle management**: Current systems store only processed text chunks. The original official document (the authoritative source) is discarded. Users cannot verify the AI-processed text against the original, undermining trust.
- **Enterprise tools are prohibitively expensive**: Multi-jurisdictional compliance monitoring platforms like C2P (Compliance & Risks) cost $50K+/year, putting systematic regulatory tracking out of reach for smaller firms, startups, and in-house teams at mid-size companies.

### Value Proposition

- **Single interface for global legal search**: Search across any configured jurisdiction in any language using natural language queries. Find laws by concept, not keyword.
- **AI-grounded legal answers**: Expose legal search as MCP tools so any LLM client can cite real, current statutes — never hallucinated references.
- **Dynamic source management**: Add new jurisdictions through configuration, not code. Administrators define crawl sources, parsing rules, and schedules via a management interface.
- **Full document provenance**: Store the original file, a clean markdown conversion, and an AI-generated summary alongside search-optimized chunks. Users can always verify against the official source.
- **Open-source and extensible**: Community-contributed source configurations can expand coverage globally.
- **Continuous regulatory monitoring**: Track changes to laws across jurisdictions over time, providing compliance officers with change histories and audit evidence of systematic monitoring — at a fraction of the cost of enterprise GRC tools.

---

## User Personas

### Primary Persona: Legal Professional

- **Demographics:** Lawyers, judges, legal researchers, law students. Age 25-65, working across 1-5 jurisdictions. Moderate tech comfort — uses web browsers daily, expects Google-like search UX. Not developers.
- **Goals:** Find relevant legal texts by concept across jurisdictions. Verify current law state. Trace authority chains (which law references which). Get reliable citations for legal work.
- **Pain Points:** Legal texts scattered across dozens of portals with different formats. Keyword search misses conceptual matches. Cross-language research is manual and slow. No way to know if a text is current or amended. Professional liability for incorrect citations.

### Secondary Personas

#### AI/LLM System (via MCP)

- **Demographics:** Claude Desktop, ChatGPT, Cursor, or any MCP-compatible LLM client.
- **Goals:** Ground legal answers in real, cited legal texts. Avoid hallucinated citations. Get summaries before full text (token efficiency).
- **Pain Points:** No reliable way to access structured legal data as tools. Existing MCP server uses stdio transport only, limiting web-based clients. Responses need structured source attribution.

#### System Administrator

- **Demographics:** DevOps/platform engineer managing the crawling infrastructure. Highly technical.
- **Goals:** Add new legal sources, monitor crawl health, handle failures without code deploys.
- **Pain Points:** Adding a source today requires writing Python code and deploying. No dashboard for crawl status. Rate limiting is global (1 req/s) instead of per-source. No alerting on failures.

#### Compliance Officer / Regulatory Affairs

- **Demographics:** Compliance professionals, regulatory affairs specialists, in-house counsel at multinational corporations. Age 30-55, tracking regulations across 2-10 markets. Uses enterprise tools daily (GRC platforms, spreadsheets). Not developers, but comfortable with structured data. *Distinct from the Legal Professional: while legal professionals perform point-in-time research to answer specific legal questions, compliance officers perform ongoing, systematic regulatory monitoring across multiple markets — their workflow is longitudinal, not episodic.*
- **Goals:** Track regulatory changes across markets that affect their company's products or operations. Build and maintain compliance matrices mapping regulations to internal controls. Get early warning when new laws are proposed or existing ones amended. Produce audit-ready evidence of regulatory monitoring.
- **Pain Points:** Manual monitoring of dozens of government gazettes and regulatory portals across countries. No unified view of regulatory obligations across jurisdictions. Compliance tools like C2P/Compliance & Risks cost $50K+/year. Regulatory changes discovered late — after enforcement deadlines. Difficulty proving to auditors that "we monitor applicable regulations."

#### Developer/Integrator

- **Demographics:** Third-party developer building legal-tech products on top of this platform.
- **Goals:** Programmatic access to legal search, document retrieval, and change tracking via stable, versioned APIs.
- **Pain Points:** Cannot build reliable integrations because: no contract guarantee (no OpenAPI spec), no stability guarantee (no versioning), no proactive notification of data changes (no webhooks), and auth is too basic for multi-tenant products.

---

## User Journey Maps

### Primary User Journey: Cross-Jurisdictional Legal Research

1. **Awareness:** Legal professional needs to research data protection obligations for a client operating in Germany and France. They know the German law (BDSG) but need to find the French equivalent.
2. **Consideration:** They currently have to visit each country's legal portal separately, search in the local language, and manually compare. They hear about this platform from a colleague or find it via search.
3. **Adoption:** They visit the platform, type "employee data protection obligations" in English, and immediately see results from GDPR, BDSG, and French CNIL guidelines — ranked by relevance with summaries.
4. **Usage:** They read summaries to triage relevant laws, click through to view the full markdown version for detailed reading, and access the original PDF/HTML when they need to cite the official source. They filter by jurisdiction to focus on France.
5. **Retention:** They bookmark the platform. When laws change, they see change tracking notifications. They start using it as their primary research starting point.

### Secondary User Journey: AI-Assisted Legal Research via MCP

1. **Awareness:** Developer configures Claude Desktop with the Legal MCP server endpoint.
2. **Adoption:** During a conversation, Claude discovers available legal search tools via `tools/list`.
3. **Usage:** User asks Claude: "What are the notice period requirements for employment termination in Germany?" Claude calls `search_laws` with the query, receives grounded results with citations, and responds with accurate legal references including statute numbers, article text, and source URLs.
4. **Retention:** Reliable citations build trust. User relies on Claude + Legal MCP for routine legal questions.

### Compliance Officer Journey: Multi-Market Regulatory Monitoring

1. **Awareness:** Compliance officer at a fintech company operating in Germany, France, Italy, and Spain needs to track payment services regulations across all four markets. Currently maintains a manual spreadsheet updated monthly by reading each country's official gazette.
2. **Consideration:** Discovers this platform can aggregate regulatory texts across EU jurisdictions and track changes automatically. Evaluates it against enterprise tools like C2P ($50K+/year).
3. **Adoption:** Sets up the platform, verifies that PSD2 transpositions (ZAG in Germany, French CMF provisions, Italian TUB, Spanish LSP) are all indexed. Runs a test search: "payment services licensing requirements" — finds relevant provisions from all four jurisdictions.
4. **Usage:** Uses change tracking (F8) to monitor when any of these laws are amended. Filters by jurisdiction to build compliance matrices. Uses the API (F7) to export relevant provisions into their GRC tool. Uses MCP integration so the company's AI assistant can answer "are we compliant with Italian payment services rules?" with grounded citations.
5. **Retention:** Reduces regulatory monitoring time from 2 days/month to 2 hours/month. When the company expands to Spain, they add BOE as a source and immediately have Spanish financial regulations searchable. Presents the platform to the board as their regulatory monitoring evidence for audit purposes.

### Tertiary User Journey: Adding a New Legal Source

1. **Awareness:** Administrator receives a request to add Swiss federal law (admin.ch) as a new source.
2. **Adoption:** Admin opens the source management UI, clicks "Add Source."
3. **Usage:** Admin configures: name ("Swiss Federal Law"), base URL, jurisdiction ("Switzerland"), crawl strategy (HTML scraping with CSS selectors), rate limit (1 req/s), schedule (daily at 03:00 UTC). Admin runs a "Test Crawl" on 5 documents to verify parsing works correctly.
4. **Retention:** Source is activated. Admin monitors crawl health on the dashboard — sees documents discovered, processed, and indexed. When EUR-Lex changes its URL structure, admin gets an alert and updates the source configuration.

### Developer Journey: Building on the Platform API

1. **Awareness:** Developer building a legal-tech product finds the platform on GitHub or discovers the API via documentation.
2. **Evaluation:** Reads the OpenAPI spec at `/api/v1/docs`. Tests a search query in the interactive documentation. Evaluates response quality, latency, and data coverage against building their own RAG pipeline.
3. **Integration:** Generates an API key, builds a prototype integration. Uses cursor-based pagination to sync legal documents into their product. Tests error handling against documented error responses.
4. **Production:** Deploys integration. Monitors API usage via their key's rate limit headers. Relies on versioned endpoints (`/api/v1/`) for stability.
5. **Expansion:** Requests additional capabilities — webhook notifications for new documents, bulk export endpoints. Contributes source configurations for jurisdictions relevant to their product.

---

## Feature Requirements

### Must Have Features

#### F1: Semantic Legal Search (including Cross-Language)

- **User Story:** As a legal professional, I want to search for legal texts using natural language describing a legal concept, so that I find relevant laws even when I don't know the exact statutory language or the language of the target jurisdiction.
- **Acceptance Criteria:**
  - [ ] Given legal texts from at least 6 jurisdictions are indexed (DE federal, DE Bayern, EU, FR, IT, ES), When a user searches "data protection rights of employees", Then results include relevant laws from each jurisdiction ranked by semantic relevance
  - [ ] Given legal texts exist in German, French, Italian, and Spanish, When a user searches in English, Then relevant results from all indexed languages are returned
  - [ ] Given results are multilingual, When displayed, Then the result language is indicated alongside each result
  - [ ] Given results are returned, When a user views a result, Then each result displays: title, jurisdiction, document type, relevance score, language, text excerpt, source URL, and last-verified date
  - [ ] Given a search is performed, When response time is measured, Then 95th percentile latency is under 2 seconds
  - [ ] Given a query with no relevant results, When the user submits it, Then the system shows "No results found" with a suggestion to broaden the search
  - [ ] Given a query contains legal citation syntax (e.g., "§ 26 BDSG"), When searched, Then the system finds the specific section

#### F2: Search Filtering

- **User Story:** As a legal professional, I want to filter search results by jurisdiction, document type, and date range, so that I only see laws relevant to my specific legal context.
- **Acceptance Criteria:**
  - [ ] Given search results exist, When the user selects a jurisdiction filter, Then only laws from that jurisdiction appear
  - [ ] Given filters are applied, When the user views filter options, Then available jurisdictions and document types are dynamically populated from actual crawled data (not hardcoded)
  - [ ] Given multiple filters are applied simultaneously, When results update, Then all filters combine with AND logic and result counts update per filter

#### F3: Document Provenance — Original, Markdown, Summary

- **User Story:** As a legal professional, I want to access the original official document, a clean readable version, and a brief summary, so that I can triage quickly and cite the authoritative source.
- **Acceptance Criteria:**
  - [ ] Given a legal text has been crawled, When the system processes it, Then three artifacts are stored: (a) original file as-is from the source, (b) markdown conversion, (c) AI-generated 2-3 sentence summary
  - [ ] Given a user views a document, When they toggle between views, Then they can switch between "Original" (rendered/downloadable), "Processed" (clean markdown), and "Summary" (brief overview)
  - [ ] Given an original is a PDF, When the user clicks "Original", Then the PDF is rendered inline or downloadable
  - [ ] Given a summary exists, When displayed, Then it includes: what the law regulates, who it applies to, and key obligations or rights
  - [ ] Given a summary is displayed, When the user views it, Then it includes an "AI-generated — verify against original" disclaimer
  - [ ] Given markdown conversion fails for a document, When the user views it, Then the original is still accessible and a "processing incomplete" indicator is shown
  - [ ] Given summary generation fails, When the document is displayed, Then it is shown without a summary, with a "summary unavailable" note

#### F4: MCP Integration (Streamable HTTP)

- **User Story:** As an AI/LLM system, I want to discover and invoke legal search tools over Streamable HTTP transport, so that I can ground legal answers in real statutory text.
- **Acceptance Criteria:**
  - [ ] Given the MCP server is running, When a client sends `tools/list`, Then it returns tool definitions for at least: `search_laws`, `get_law_by_id`, `get_sources`, `get_summary`
  - [ ] Given a client calls `search_laws`, When results are returned, Then each result includes: citation-ready reference, summary, relevance score, and source URL
  - [ ] Given the MCP transport is Streamable HTTP, When any HTTP-capable MCP client connects to `/mcp`, Then the JSON-RPC 2.0 protocol is fully supported
  - [ ] Given multiple LLM clients connect concurrently, When 100 sessions are active, Then 95th percentile response latency remains under 3 seconds and zero sessions receive errors due to resource exhaustion
  - [ ] Given a client calls `search_laws` with an invalid or empty query, When the request is processed, Then a JSON-RPC error response with a descriptive error code and message is returned
  - [ ] Given the MCP endpoint, When any client connects, Then no authentication is required (open access for v1; API key auth deferred to v2)

#### F5: Dynamic Source Management

- **User Story:** As a system administrator, I want to add, configure, and manage crawl sources through a UI, so that the system can crawl new jurisdictions without code changes.
- **Acceptance Criteria:**
  - [ ] Given an admin is authenticated, When they open source management, Then they see a list of all configured sources with status (active/paused/error), last crawl time, and document count
  - [ ] Given an admin adds a new source, When they fill in the configuration (name, URL, jurisdiction, strategy, schedule, rate limit), Then the source is saved and enters the crawl queue
  - [ ] Given a source is configured, When the admin clicks "Test Crawl", Then the system performs a dry-run crawl of up to 5 documents and displays the results for validation
  - [ ] Given a source requires authentication, When the admin configures it, Then credential storage is supported (API keys, login credentials)
  - [ ] Given a source configuration changes, When saved, Then the next crawl uses the updated configuration without restart

#### F6: Automated Crawling Pipeline

- **User Story:** As a system administrator, I want crawlers to automatically discover, fetch, parse, and index legal documents on a schedule, so that the search index stays current without manual intervention.
- **Acceptance Criteria:**
  - [ ] Given a source is active and scheduled, When the scheduled time arrives, Then the crawler runs automatically
  - [ ] Given a document has already been crawled, When its content hasn't changed (same content hash), Then it is skipped to avoid redundant processing
  - [ ] Given a document has changed since the last crawl, When re-crawled, Then the original file is re-stored, markdown is regenerated, summary is updated, and RAG chunks are re-embedded
  - [ ] Given a crawl encounters an error (network timeout, rate limit, parse failure), When the error occurs, Then it is logged, the document is skipped, and the crawl continues with remaining documents
  - [ ] Given per-source rate limits are configured, When the crawler operates, Then it never exceeds the configured requests per second for that source
  - [ ] Given a crawl is already running for a source, When the next scheduled run triggers, Then it is skipped and logged as "skipped: previous run still in progress"

#### F7: REST API

- **User Story:** As a developer, I want to access legal search and document retrieval via a versioned REST API, so that I can build applications on top of this platform.
- **Acceptance Criteria:**
  - [ ] Given the API is deployed, When a developer accesses `/api/v1/docs`, Then an OpenAPI specification with interactive documentation is available
  - [ ] Given a developer has an API key, When they make authenticated requests, Then rate limiting is enforced per API key
  - [ ] Given a list endpoint, When the developer requests it, Then cursor-based pagination is supported
  - [ ] Given search, document retrieval, source listing, and change tracking endpoints exist, When called, Then all return consistent JSON response shapes
  - [ ] Given any API error occurs, When the response is returned, Then it uses a consistent error shape: `{error: {code, message, details}}` with appropriate HTTP status codes (400, 401, 404, 429, 500)

#### F8: Change Tracking

- **User Story:** As a legal professional or compliance officer, I want to see what has changed in laws I care about, so that I stay current with legislative developments and can demonstrate regulatory monitoring.
- **Acceptance Criteria:**
  - [ ] Given a law has been crawled multiple times, When the user views it, Then a "Change History" section shows: date of change, sections modified, and change type (new/modified/repealed)
  - [ ] Given a law is repealed, When crawled, Then it is marked as repealed with effective date but remains searchable
  - [ ] Given a law is marked as repealed, When it appears in search results, Then it is visually distinguished (e.g., "REPEALED" badge) and ranked lower than active laws
  - **Dependency:** Requires F6 (Automated Crawling Pipeline) to detect changes via content hash comparison during scheduled re-crawls.

### Should Have Features

#### F9: Crawl Health Dashboard

- **User Story:** As a system administrator, I want to see real-time crawl status and error logs, so that I can quickly identify and resolve issues with data freshness.
- **Acceptance Criteria:**
  - [ ] Given crawls are running, When the admin opens the dashboard, Then per-source status is visible: last crawl time, documents found/new/updated/failed, next scheduled crawl
  - [ ] Given a crawl fails, When the failure is detected, Then an alert is generated (configurable: webhook, email)
  - [ ] Given a source has been unreachable for > 48 hours, When the admin views the dashboard, Then a "Stale" warning indicator is shown

#### F10: PDF Upload and OCR

- **User Story:** As a legal professional, I want to upload legal PDFs that aren't available from crawled sources, so that they become searchable alongside public legal texts.
- **Acceptance Criteria:**
  - [ ] Given a user uploads a PDF, When processing completes, Then the system stores the original, runs OCR if needed, converts to markdown, generates a summary, and creates RAG chunks
  - [ ] Given a scanned PDF (image-only), When OCR runs, Then text is extracted and low-confidence sections are flagged
  - [ ] Given an upload exceeds 100 pages, When processing starts, Then it runs asynchronously with a progress indicator

### Could Have Features

#### F11: Legal Citation Graph

- **User Story:** As a legal professional, I want to see which laws reference each other, so that I can trace legal authority chains.
- **Acceptance Criteria:**
  - [ ] Given a law references other statutes, When the user views it, Then referenced laws are linked and navigable

#### F12: Saved Searches and Notifications

- **User Story:** As a legal professional, I want to save a search and be notified when new matching laws are added or existing ones change.
- **Acceptance Criteria:**
  - [ ] Given a user saves a search, When new results match the query, Then the user receives a notification

#### F13: Community Source Contributions

- **User Story:** As a domain expert, I want to submit a source configuration for a jurisdiction I know well, so that the platform's coverage expands through community effort.
- **Acceptance Criteria:**
  - [ ] Given a user submits a source config, When an admin reviews and approves it, Then the source is added to the crawl queue

#### F14: Bilingual User Interface (EN + DE)

- **User Story:** As a user in a German-speaking or English-speaking context, I want the interface in my language, so that I can navigate and use the platform without language barriers.
- **Acceptance Criteria:**
  - [ ] Given a user selects German, When any UI page loads, Then all navigation, labels, buttons, and system messages are displayed in German
  - [ ] Given a user selects English, When any UI page loads, Then all UI elements are displayed in English
  - [ ] Given the admin UI, When an admin switches language, Then source management, dashboard, and all admin-facing text are also translated
  - [ ] Given the i18n framework, When a new UI string is added during development, Then it requires both EN and DE translations

### Won't Have (This Phase)

- **User accounts and multi-tenancy**: Anonymous search + API key access for v1. Full user accounts deferred to v2.
- **Real-time legal alerts/webhooks**: Push notifications for law changes are deferred. Polling-based change tracking via API is in scope.
- **Legal analysis/interpretation AI**: The system retrieves and cites law — it does not interpret or give legal advice.
- **Mobile native apps**: Web-responsive only for v1.
- **Offline access**: Requires internet connectivity.
- **Fine-tuned embedding models**: Use commercial embedding APIs with multilingual support. Domain-specific fine-tuning is deferred.
- **Compliance matrix builder**: Compliance officers can use search + filtering + API export as building blocks. A dedicated matrix UI is deferred to v2.
- **Audit-ready reporting**: Formal audit trail and evidence reports deferred to v2. v1 provides change tracking (F8) and API access (F7) as building blocks.
- **Case law indexing**: v1 focuses on statutes and regulations. Court decisions and case law indexing deferred to v2.
- **Annotation and commenting on legal texts**: In-document annotation deferred to v2.
- **SSO/SAML enterprise authentication**: v1 uses API keys. Enterprise SSO deferred to v2.
- **Visual amendment timelines**: Timeline visualization of law history deferred to v2.

### Feature Dependency Map

The following diagram shows which features depend on others. Implementation should follow this order:

```
F5 (Dynamic Source Management)
 └──▸ F6 (Automated Crawling Pipeline)
       ├──▸ F3 (Document Provenance)  ──▸ F1 (Semantic Search) ──▸ F2 (Search Filtering)
       ├──▸ F8 (Change Tracking)
       └──▸ F9 (Crawl Health Dashboard)

F1 (Semantic Search) ──▸ F4 (MCP Integration)
F1 (Semantic Search) ──▸ F7 (REST API)
F10 (PDF Upload) depends on F3 pipeline (store original → markdown → summary → chunk → embed)
F14 (Bilingual UI) is cross-cutting — applies to all frontend features
```

**Critical path**: F5 → F6 → F3 → F1 → F4/F7. Source management and crawling must exist before documents can be processed, searched, or served via MCP/API.

---

## Detailed Feature Specifications

### Feature: Dynamic Source Management (F5)

**Description:** The source management system allows administrators to define, configure, test, and manage legal text crawl sources entirely through a web interface. Each source represents a government legal portal or document repository. Sources are defined by their URL structure, parsing strategy, scheduling, and rate limiting parameters. The system supports multiple crawl strategies to handle different source formats (HTML scraping, XML parsing, API/SPARQL endpoints, PDF repositories).

**User Flow:**

1. Admin authenticates and navigates to Source Management.
2. Admin clicks "Add Source" and fills in the configuration form:
   - **Basic info**: Name, description, jurisdiction (country/region), language
   - **Crawl target**: Base URL, URL discovery method (sitemap, pagination, RSS feed, API)
   - **Parsing**: Content extraction strategy (CSS selectors for HTML, XPath for XML, API response mapping)
   - **Scheduling**: Cron expression or interval (hourly/daily/weekly)
   - **Rate limiting**: Max requests per second, concurrent request limit
   - **Optional**: Authentication credentials, custom headers, robots.txt override justification
3. Admin clicks "Test Crawl" — system fetches and parses 5 sample documents, displaying results.
4. Admin reviews test results, adjusts configuration if needed.
5. Admin activates the source — it enters the scheduled crawl queue.
6. System crawls on schedule, processing documents through the full pipeline: fetch → store original → convert to markdown → generate summary → chunk → embed → index.

**Business Rules:**

- Rule 1: Every source must have a successful test crawl before it can be activated.
- Rule 2: Rate limits are enforced per-source (not globally). The system never exceeds the configured rate for a given source.
- Rule 3: robots.txt is respected by default. Override requires admin justification that is logged.
- Rule 4: Source configurations are versioned — changes are tracked with timestamps and the admin who made them.
- Rule 5: Deleting a source does not delete already-crawled documents. Documents retain their source attribution but are marked as "source inactive."
- Rule 6: A source can be paused (stops crawling but retains config and data) or deactivated (stops crawling, hides from search results).

**Edge Cases:**

- Source requires JavaScript rendering → v1: Not supported. Log a clear error. Future: headless browser integration.
- Source returns HTTP 429 (rate limited) → Back off exponentially, retry up to 3 times, then mark the crawl run as partial and alert admin.
- Source URL structure changes (404 spike) → If > 10% of URLs for a source return errors in a single crawl run, pause the source and alert admin.
- Source returns different HTML structure than configured → Parse failure is logged per-document; crawl continues with remaining documents.
- Two sources provide the same law (e.g., EU Regulation from EUR-Lex and a national transposition portal) → Both are stored with distinct source attribution. Deduplication is by canonical document identifier where available (e.g., CELEX number for EU law).

---

## Success Metrics

### Key Performance Indicators

- **Coverage:** 6 jurisdictions (Bundesrecht, Bayern.Recht, EUR-Lex, Legifrance, Normattiva, BOE) with 10,000+ indexed legal documents within 3 months of launch
- **Search quality (at launch):** Mean Reciprocal Rank (MRR) > 0.7 on a legal domain evaluation set; zero-result rate < 5%. *Evaluation methodology: a gold-standard test set of ≥ 50 legal queries with human-judged relevant documents, stratified across jurisdictions and languages. MRR is computed over the top-10 results per query.*
- **Freshness (ongoing, after initial ramp-up):** 95% of active sources have been successfully crawled within their configured schedule
- **MCP adoption:** 50+ active MCP sessions per week within 6 months
- **API adoption:** 20+ active API keys processing requests within 6 months
- **Change tracking adoption:** 10+ unique users tracking changes across 2+ jurisdictions within 6 months
- **Availability (from month 2):** Search API uptime > 99.5% monthly (managed hosted instance; self-hosted uptime is the operator's responsibility)

### Tracking Requirements

| Event | Properties | Purpose |
|-------|------------|---------|
| search_performed | query, jurisdiction_filter, result_count, latency_ms | Measure search usage and quality |
| search_result_clicked | query, result_rank, document_id, view_type (summary/markdown/original) | Measure result relevance and user preference |
| mcp_tool_called | tool_name, session_id, latency_ms, result_count | Measure MCP adoption and performance |
| source_added | source_name, jurisdiction, strategy_type | Track coverage growth |
| source_crawl_completed | source_id, documents_found, documents_new, documents_updated, documents_failed, duration_s | Monitor crawl health |
| source_crawl_failed | source_id, error_type, retry_count | Track reliability |
| document_processed | document_id, source_id, has_original, has_markdown, has_summary, chunk_count | Track pipeline completeness |
| api_request | endpoint, api_key_id, latency_ms, status_code | Track API usage patterns |
| change_history_viewed | document_id, jurisdiction, changes_count | Track change tracking adoption and compliance monitoring usage |

---

## Constraints and Assumptions

### Constraints

- **Open source**: The system is open-source. No proprietary dependencies that prevent community deployment.
- **Technology**: Backend rewrite uses Rust (crawler, API, RAG/MCP server). The Go-based cgpt-rag-mcp-api was studied as a reference for RAG patterns (pgvector, MCP transport, embedding interfaces) but is not part of the production architecture. This constrains the backend contributor pool to Rust developers.
- **Team size**: Small team (1-3 engineers). v1 scope must be achievable within this capacity.
- **Deployment**: Docker-based, targeting GCP or Azure container hosting (EU region). PostgreSQL + pgvector as the primary data store.
- **Bilingual UI**: English and German from day one via i18n framework. English is the primary/default language. (See F14 for acceptance criteria.)
- **Embedding API costs**: Cloud embedding APIs charge per token. No hard budget ceiling, but costs must be managed through batching, caching, and incremental processing. Estimated v1 cost for 10K documents (~500K chunks): ~$10-50/month depending on provider.
- **AI costs**: Summary generation uses cloud LLM APIs with per-call costs. v1 must document expected cost per 1,000 documents processed.
- **Government website rate limits**: Legal sources are government portals that may rate-limit or block aggressive crawlers. Per-source rate limiting is mandatory.
- **Legal text copyright**: Legal texts are public domain in most jurisdictions (e.g., Section 5(1) UrhG in Germany, 17 USC 105 in the US), but this varies. Each source addition requires copyright verification.
- **Secrets management**: Source credentials (F5) must be encrypted at rest. Community deployments must support bring-your-own secret management (env vars, Vault, etc.).
- **Embedding model limitations**: Current multilingual embedding models may have lower precision for legal terminology in some languages compared to domain-specific models.
- **pgvector scaling**: HNSW indexes work well up to ~20M vectors per table. Beyond that, partitioning or alternative vector stores may be needed.
- **v1 scope**: 6 sources across 4 languages (DE, FR, IT, ES + EU English). US expansion deferred to v2.

### Assumptions

- **Legal texts remain publicly accessible**: Government portals will continue providing machine-readable legal texts in parseable formats (HTML, XML, PDF with selectable text, or API). JavaScript-only rendering and CAPTCHA are not supported in v1. If a source removes access, existing data is retained but not updated.
- **Cloud embedding APIs are reliable**: Vertex AI / OpenAI embedding endpoints are available with acceptable latency (< 200ms per call). If unavailable, search degrades to keyword-only. Interface abstraction allows swapping providers or falling back to local models.
- **Multilingual embeddings are sufficient**: A single multilingual embedding model provides acceptable search quality across all v1 languages (DE, FR, IT, ES, EN). If cross-language recall drops below MRR 0.5 for any language, per-language models with a language-routing layer will be evaluated.
- **pgvector is sufficient for v1 scale**: pgvector provides acceptable search latency (<500ms p95) and recall at the expected document scale (100K–1M chunks in v1). If not, migration to a dedicated vector database (Qdrant, Weaviate) is planned.
- **Administrators are technical**: Source configuration requires understanding of HTML/CSS selectors, URL patterns, and rate limiting concepts.
- **Legal professionals trust AI-generated summaries with disclaimers**: Summaries are marked as AI-generated and users understand they must verify against the original.

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Cross-language search quality is poor for legal terminology | High | Medium | Benchmark with legal-domain test set before launch. Allow per-language model override via interface abstraction. |
| Government sources rate-limit or block crawlers | High | Medium | Identify as a legal research tool in User-Agent. Respect robots.txt. Contact source operators proactively. Per-source rate limiting. |
| Scope creep from "3 sources" to "worldwide" delays v1 | High | High | Strict v1 scope: 6 sources across 4 languages (DE/FR/IT/ES + EU). Existing 3 (Bundesrecht, Bayern.Recht, EUR-Lex) + 3 new EU (Legifrance, Normattiva, BOE). US expansion deferred to v2. "Worldwide" is the architecture, not the v1 content. |
| OCR quality insufficient for legal text accuracy | Medium | Medium | Use high-quality OCR (Mistral Document AI). Flag low-confidence sections. Store original alongside processed text so users can verify. |
| pgvector cannot handle scale at 15M+ chunks | Medium | Low | Monitor performance. Partition by jurisdiction. Have migration path to dedicated vector DB (Qdrant, Weaviate) if needed. |
| AI-generated summaries contain inaccuracies | High | Medium | Clear "AI-generated" disclaimers. Summaries include caveat: "Verify against original text." Optional curator review queue. |
| Source format changes break crawlers | Medium | High | Crawl health monitoring with alerts. Per-document error logging (don't fail entire crawl). Admin can update source config without code deploy. |
| Two-service architecture adds operational complexity | Medium | Medium | Clear service boundaries. Shared database reduces coupling. Comprehensive health checks and monitoring. |
| Full rewrite takes longer than expected, delaying all new functionality | High | High | Phased approach: (1) build new crawler + source management alongside existing system, (2) migrate search + MCP, (3) decommission Python services. Keep existing system operational until feature parity. |
| Embedding API becomes unavailable or pricing changes dramatically | Medium | Low | Interface abstraction allows swapping providers. Local embedding models (e.g., sentence-transformers) as fallback. |
| AI-generated legal summaries may create liability if relied upon for legal decisions | High | Low | Prominent disclaimers. Summaries clearly marked as AI-generated and non-authoritative. Terms of service disclaim legal advice. |
| Silent data corruption in the processing pipeline degrades search quality | Medium | Medium | Pipeline quality checks: validate non-empty markdown, chunk count within expected range, embedding dimensions correct. `document_processed` tracking event captures pipeline completeness. |

---

## Open Questions

> All questions resolved — decisions captured below.

- [x] **Q1: v1 Jurisdiction Scope** — **RESOLVED**: EU expansion first. v1 includes existing 3 (Bundesrecht, Bayern.Recht, EUR-Lex) + 3 new EU sources (French Legifrance, Italian Normattiva, Spanish BOE). US expansion deferred to v2. Compliance officer is a key persona driving the multi-market requirement.
- [x] **Q2: Content Curation Workflow** — **RESOLVED**: Auto-publish for v1. No human review required. Content goes live after passing automated quality checks (successful parse, non-empty markdown, summary generated). Curator review queue deferred to v1.1.
- [x] **Q3: Hosting and Data Residency** — **RESOLVED**: Docker-based deployment, hosted in GCP or Azure containers (EU region). PostgreSQL + pgvector as the database. No strict data residency requirement beyond hosting in EU.
- [x] **Q4: Embedding Cost Ceiling** — **RESOLVED**: No hard ceiling on embedding API costs. Budget managed through batching, caching, and incremental processing — but not a blocking constraint for v1.
- [x] **Q5: Frontend Language** — **RESOLVED**: Bilingual from day one — English (EN) and German (DE). i18n framework from the start, not retrofitted. English is the primary/default language.

---

## Supporting Research

### Competitive Analysis

| Platform | Coverage | Search Type | MCP Support | Open Source | Dynamic Sources |
|----------|----------|-------------|-------------|-------------|----------------|
| **C2P / Compliance & Risks** | 195 countries, 70K+ regulations | Keyword + taxonomies | No | No (enterprise SaaS, $50K+/yr) | Yes (curated by analysts) |
| **EUR-Lex** | EU only | Keyword | No | No (government portal) | No |
| **Gesetze-im-Internet** | German federal only | Keyword | No | No (government portal) | No |
| **Google Scholar (Legal)** | Global case law | Keyword + citation | No | No | No |
| **Westlaw/LexisNexis** | Global | Keyword + headnotes | No | No (proprietary, $$$) | No |
| **This Platform** | Dynamic, global (v1: EU) | Semantic (vector) | Yes (Streamable HTTP) | Yes | Yes |

**Differentiation**: No existing platform combines semantic legal search + MCP integration + dynamic source management + open source. C2P (Compliance & Risks) is the closest competitor for multi-jurisdictional regulatory monitoring, covering 195 countries with human-curated content — but it costs $50K+/year, uses keyword search, has no AI/MCP integration, and is closed-source. Our differentiation: (1) semantic search finds conceptual matches that keyword search misses, (2) MCP integration enables AI assistants to cite real law, (3) open-source means community-driven source expansion and zero licensing cost, (4) compliance officers get C2P-like regulatory monitoring at a fraction of the cost.

### User Research

Based on analysis of the current system's design decisions, legal tech market reports, and known industry pain points:
- Legal professionals report spending significant time on legal research — industry estimates suggest 20-30% of billable hours (varies by practice area and firm size)
- Cross-jurisdictional legal research is widely recognized as a major pain point, with practitioners reporting difficulty finding relevant law across borders
- LLM hallucination of legal citations is a documented concern — Stanford HAI's 2023 study found GPT-4 hallucinated legal citations in a majority of tested cases
- MCP adoption is accelerating — Claude Desktop, Cursor, and other AI tools are integrating MCP clients
- Government open data initiatives are making legal texts increasingly machine-readable

### Market Data

- Global legal tech market: $29.5B (2024), projected $69.7B by 2032 (Allied Market Research)
- Legal AI segment growing at 28.5% CAGR
- Open legal data movement accelerating: EU Open Data Directive, US USLM XML, UK legislation.gov.uk API
- MCP ecosystem: 100+ MCP servers published, major AI tools adding MCP client support
