# Legal MCP Server

Ein Model Context Protocol (MCP) HTTP-Server für semantische Rechtssuche über bayerisches, deutsches und EU-Recht.

## Features

- **Semantische Suche** mit German Legal BERT (deepset/gbert-base)
- **PostgreSQL + pgvector** für vektorbasierte Suche
- **FastAPI** mit async/await
- **MCP Protocol** für LLM-Integration
- **REST API** für Frontend-Integration
- **5 MCP Tools**: search_laws, get_law_by_id, search_case_law, get_legal_changes, get_related_laws

## Installation

```bash
# Dependencies installieren
pip install -r requirements.txt

# Environment variables setzen
cp .env.example .env

# Datenbank initialisieren (mit Docker)
docker-compose up -d postgres
```

## Verwendung

### Server starten

```bash
# Development
uvicorn src.main:app --reload --port 8000

# Production
uvicorn src.main:app --host 0.0.0.0 --port 8000 --workers 4
```

### MCP Server (stdio)

```bash
# Für MCP stdio transport
python -m src.mcp.protocol
```

### API Endpoints

- `GET /` - Service Info
- `GET /api/health` - Health Check
- `POST /api/search` - Gesetze suchen
- `GET /api/laws/{id}` - Gesetz abrufen
- `GET /api/laws/abbreviation/{abbr}` - Gesetz per Kürzel
- `POST /api/search/case-law` - Rechtsprechung suchen
- `POST /api/changes` - Rechtliche Änderungen
- `POST /api/related` - Verwandte Gesetze
- `POST /mcp` - MCP HTTP Endpoint (in Entwicklung)

## MCP Tools

### 1. search_laws

```json
{
  "query": "Datenschutz",
  "jurisdiction": ["federal", "eu"],
  "document_type": ["law", "regulation"],
  "include_case_law": false,
  "limit": 10
}
```

### 2. get_law_by_id

```json
{
  "abbreviation": "BGB"
}
```

### 3. search_case_law

```json
{
  "query": "Mietrecht Kündigung",
  "jurisdiction": ["federal"],
  "court_level": ["bgh"],
  "limit": 10
}
```

### 4. get_legal_changes

```json
{
  "days": 30,
  "jurisdiction": ["federal"],
  "limit": 50
}
```

### 5. get_related_laws

```json
{
  "law_id": "uuid-here",
  "limit": 5
}
```

## Entwicklung

```bash
# Tests ausführen
pytest

# Code formatieren
black src/ tests/

# Linting
ruff src/ tests/

# Type checking
mypy src/
```

## Technische Details

- **Python**: 3.11+
- **Framework**: FastAPI 0.109+
- **Database**: PostgreSQL 15 + pgvector
- **ML**: sentence-transformers (German Legal BERT)
- **ORM**: SQLAlchemy 2.0 (async)
- **MCP**: mcp-python SDK

## API Authentifizierung

API Key im Header:

```bash
curl -H "X-API-Key: dev_key_12345" http://localhost:8000/api/search
```

Default Development Key: `dev_key_12345`

## Deployment

```bash
# Docker Build
docker build -t legalmcp-server .

# Docker Run
docker run -p 8000:8000 \
  -e POSTGRES_HOST=postgres \
  -e POSTGRES_PASSWORD=password \
  legalmcp-server
```

## Lizenz

MIT
