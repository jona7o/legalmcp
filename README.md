# Legal MCP Server

Ein Model Context Protocol (MCP) HTTP-Server für semantische Rechtssuche über bayerisches, deutsches und EU-Recht.

## Features

- 🔍 **Semantische Suche** mit German Legal BERT Embeddings
- 🇩🇪 **Bundesrecht** (Gesetze-im-Internet)
- 🇪🇺 **EU-Recht** (EUR-Lex)
- 🥨 **Bayerisches Landesrecht** (Bayern.Recht)
- ⚖️ **Rechtsprechung** von bayerischen und Bundesgerichten
- 🤖 **MCP Protocol** für LLM-Integration
- 💬 **Gemini Chat** über Vertex AI
- 🔄 **Auto-Crawler** für tägliche Updates

## Architektur

```
├── packages/
│   ├── crawler/          # Python Crawler Service
│   ├── mcp-server/       # Python MCP HTTP Server + FastAPI
│   └── frontend/         # Next.js Frontend mit shadcn/ui
├── docker-compose.yml    # Lokale Entwicklung
└── .github/workflows/    # CI/CD
```

## Quick Start (Lokal mit Docker)

```bash
# Alle Services starten
docker-compose up

# Frontend: http://localhost:3000
# MCP Server: http://localhost:8000
# PostgreSQL: localhost:5432
```

## Tech Stack

- **Backend**: Python 3.11, FastAPI, SQLAlchemy, pgvector
- **Frontend**: Next.js 14, TypeScript, shadcn/ui, Tailwind
- **Database**: PostgreSQL 15 + pgvector Extension
- **ML**: German Legal BERT (deepset/gbert-base)
- **LLM**: Vertex AI Gemini 1.5 Pro
- **Deployment**: Cloud Run, Firebase Hosting

## Development

### Prerequisites

- Docker & Docker Compose
- Python 3.11+
- Node.js 18+
- pnpm

### Lokale Entwicklung

```bash
# 1. Repository klonen
git clone <repo-url>
cd legalmcp

# 2. Environment Setup
cp .env.example .env

# 3. Docker Services starten
docker-compose up -d postgres

# 4. Crawler ausführen (initial seed)
cd packages/crawler
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python -m src.main

# 5. Backend starten
cd packages/mcp-server
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
uvicorn src.main:app --reload

# 6. Frontend starten
cd packages/frontend
pnpm install
pnpm dev
```

## Rechtliche Hinweise

⚠️ **Kein Ersatz für Rechtsberatung**: Dieses Tool dient nur zu Informationszwecken. Die Inhalte ersetzen keine anwaltliche Beratung.

Amtliche Rechtstexte sind gemäß § 5 Abs. 1 UrhG gemeinfrei und können frei verwendet werden.

## Lizenz

MIT
