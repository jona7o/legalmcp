# Specification: 001-global-legal-crawler-mcp-rag

## Status

| Field             | Value          |
| ----------------- | -------------- |
| **Created**       | 2026-02-19     |
| **Current Phase** | COMPLETE       |
| **Last Updated**  | 2026-02-21     |

## Documents

| Document                | Status      | Notes |
| ----------------------- | ----------- | ----- |
| product-requirements.md | completed   | v1.2 — approved by user after 3-agent review |
| solution-design.md      | completed   | v1.0 — all 14 sections complete |
| implementation-plan.md  | completed   | v1.0 — 8 phases, 32 tasks, full TDD structure |

**Status values**: `pending` | `in_progress` | `completed` | `skipped`

## Decisions Log

| Date       | Decision                          | Rationale |
| ---------- | --------------------------------- | --------- |
| 2026-02-19 | Start with full PRD → SDD → PLAN | User chose recommended path for comprehensive rewrite |
| 2026-02-19 | Go RAG reference repo accessible  | User cloned to /home/tj/dev/innfactoryai/cgpt-rag-mcp-api — full analysis completed |
| 2026-02-19 | v1 jurisdictions: EU expansion    | Existing 3 (Bundesrecht, Bayern.Recht, EUR-Lex) + 3 new EU (Legifrance, Normattiva, BOE). US deferred to v2. |
| 2026-02-19 | Compliance Officer as key persona | User identified compliance/regulatory affairs as the missing persona. Drives multi-market monitoring use case. |
| 2026-02-19 | Auto-publish for v1               | No human review required. Curator review deferred to v1.1. |
| 2026-02-19 | Docker on GCP/Azure (EU region)   | PostgreSQL + pgvector. No strict data residency beyond EU hosting. |
| 2026-02-19 | No embedding budget ceiling       | Costs managed through batching/caching, but not a blocking constraint. |
| 2026-02-19 | Bilingual EN+DE from day one      | English primary, German secondary. i18n framework from start, not retrofitted. |
| 2026-02-19 | C2P as key competitor reference    | C2P by Compliance & Risks (195 countries, $50K+/yr) — our open-source differentiator. |
| 2026-02-19 | Rust-only backend                 | Go RAG repo was reference only for patterns. Production backend is 100% Rust (Axum). |
| 2026-02-19 | F8 Change Tracking → Must Have    | Promoted from Should Have. Two user journeys (Legal Professional, Compliance Officer) depend on change tracking. |
| 2026-02-20 | Two binaries + async job queue (apalis)  | crawler-service and api-service are separate Rust binaries sharing a Cargo workspace. apalis (PostgreSQL-backed) decouples ingestion from crawl trigger. |
| 2026-02-20 | sqlx 0.8 + pgvector crate                | Compile-time verified queries, fully async, native Vector type support for cosine search. SeaORM still RC, Diesel sync-first. |
| 2026-02-20 | HNSW index, single chunks table          | HNSW is self-maintaining, better recall. Single unified table filtered by metadata — no per-jurisdiction table proliferation. |
| 2026-02-20 | Object storage (object_store crate)      | Original files (PDF/HTML/XML) in GCS or Azure Blob via apache/object_store. Avoids bloating PostgreSQL with binary data. |
| 2026-02-20 | Mistral Document AI for PDF extraction   | Native Rust PDF extraction (pdf-extract/lopdf) too immature for legal PDFs. Mistral returns structured markdown. |
| 2026-02-20 | rmcp 0.16 (official Rust MCP SDK)        | Supports Streamable HTTP + stdio natively. No hand-rolled JSON-RPC needed. |
| 2026-02-20 | Vertex AI text-embedding-004 via Embedder trait | 768-dim multilingual. Trait abstraction allows provider swap. Fixes current mismatch (crawler used local model, API used Vertex AI). |
| 2026-02-20 | tokio-cron-scheduler 0.15               | PostgreSQL-backed persistence, cron syntax, no Redis dependency. Adequate for single crawler instance. |
| 2026-02-20 | Unified documents + chunks schema        | Single documents table with document_type discriminator. Handles PDF uploads and new source types without schema changes. |
| 2026-02-20 | 5-crate Cargo workspace                  | crates/{core, ingest, embeddings, crawler, api}. core and ingest shared by both binaries. |
| 2026-02-20 | next-intl 4.x for bilingual UI           | Native App Router + RSC support. next-i18next is Pages Router-first with App Router workarounds. |
| 2026-02-20 | SHA-256 hash + document_versions table  | Content hash comparison on each crawl. Changed docs get new version row. Repeals detected via 404 or XML marker. |

## Context

**Vision**: Rewrite the existing Legal MCP platform from a Germany-focused Python monorepo into a global legal crawler & MCP RAG system with clean architecture.

**Key Changes from Current State**:
- **Backend**: Python (FastAPI) → Rust (Axum). Go RAG repo studied for patterns only.
- **Frontend**: Keep Next.js + shadcn/ui (TypeScript), bilingual EN+DE
- **Scope**: Expand from 3 German/EU legal sources to 6 EU sources (v1), then US and worldwide (v2+)
- **Storage**: Store original files, markdown conversions, and brief summaries alongside RAG chunks
- **Deployment**: Docker-based, GCP or Azure containers (EU region)

**Current State (being replaced)**:
- Python monorepo: crawler, mcp-server (FastAPI), frontend (Next.js), postgres (pgvector)
- 3 crawlers: Bundesrecht (XML), Bayern.Recht (HTML), EUR-Lex (SPARQL)
- PostgreSQL + pgvector for storage + vector search
- Google Vertex AI for embeddings + Gemini for chat
- MCP via stdio only

**Reference**: innFactory-AI/cgpt-rag-mcp-api (Go RAG approach — fully analyzed)

---

_This file is managed by the specification-management skill._
