# Legal MCP Crawler

Python-basierter Crawler Service für deutsche und europäische Rechtstexte.

## Features

- **3 Datenquellen**:
  - 🇩🇪 Bundesrecht (Gesetze-im-Internet XML)
  - 🥨 Bayern.Recht (Web Scraping)
  - 🇪🇺 EUR-Lex (SPARQL + HTML)

- **Intelligente Verarbeitung**:
  - Change Detection via SHA-256 Content Hash
  - Text Chunking (max 512 tokens)
  - German Legal BERT Embeddings
  - Async Crawling mit Rate Limiting

- **Produktionsreif**:
  - Retry-Logik mit exponential backoff
  - Structured Logging
  - Progress Reporting
  - Error Handling
  - PostgreSQL + pgvector

## Installation

```bash
cd packages/crawler

# Virtual Environment erstellen
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# Dependencies installieren
pip install -r requirements.txt
```

## Konfiguration

Umgebungsvariablen in `.env` (siehe `.env.example`):

```env
# Database
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=legalmcp
POSTGRES_USER=legal
POSTGRES_PASSWORD=your_password

# Crawler Settings
CRAWLER_RATE_LIMIT=1.0
CHUNK_SIZE=512
CHUNK_OVERLAP=50

# Embedding Model
EMBEDDING_MODEL=deepset/gbert-base

# Enable/Disable Sources
ENABLE_BUNDESRECHT=true
ENABLE_BAYERN=true
ENABLE_EURLEX=true
```

## Verwendung

### Standalone

```bash
# Alle aktivierten Crawler ausführen
python -m src.main

# Logs ansehen
tail -f crawler.log
```

### Mit Docker

```bash
# Crawler einmalig ausführen
docker-compose run --rm crawler

# Im Hintergrund mit auto-restart
docker-compose up -d crawler
```

### Programmatisch

```python
import asyncio
from src.sources import BundesrechtCrawler
from src.db import db

async def main():
    await db.init_db()
    
    async with BundesrechtCrawler() as crawler:
        stats = await crawler.crawl()
        print(f"Crawled: {stats}")
    
    await db.close()

asyncio.run(main())
```

## Architektur

```
src/
├── main.py              # Orchestrator
├── config.py            # Konfiguration
├── sources/
│   ├── base.py          # Abstract Crawler
│   ├── bundesrecht.py   # Bundesrecht Crawler
│   ├── bayern.py        # Bayern.Recht Crawler
│   └── eurlex.py        # EUR-Lex Crawler
├── processing/
│   ├── parser.py        # XML/HTML Parsing
│   ├── normalizer.py    # Text Normalisierung
│   ├── chunker.py       # Text Chunking
│   └── embedder.py      # Embedding Generation
└── db/
    ├── models.py        # SQLAlchemy Models
    └── operations.py    # DB Operations
```

## Crawler Details

### Bundesrecht (Gesetze-im-Internet)

- **Quelle**: https://www.gesetze-im-internet.de/gii-toc.xml
- **Format**: XML
- **Umfang**: ~5.000 Bundesgesetze und -verordnungen
- **Update-Frequenz**: Täglich empfohlen

### Bayern.Recht

- **Quelle**: https://www.gesetze-bayern.de
- **Format**: HTML (Web Scraping)
- **Rate Limit**: 1 req/sec (respektvoll!)
- **Umfang**: ~1.000 bayerische Landesgesetze
- **Hinweis**: Robots.txt beachten

### EUR-Lex

- **Quelle**: SPARQL Endpoint + CELLAR API
- **Format**: SPARQL JSON + HTML
- **Umfang**: EU-Verordnungen, -Richtlinien, -Beschlüsse
- **Sprache**: Deutsch
- **Filter**: Ab 2020

## Datenmodell

### Document
- Metadaten (Titel, Typ, Jurisdiktion)
- Volltext + Content Hash
- URLs (Quelle, XML, PDF)
- Timestamps (created_at, crawled_at)

### DocumentChunk
- Text-Segmente (max 512 tokens)
- Embeddings (768-dim für GBERT)
- Abschnittsinformationen
- Metadaten (Titel, Nummer)

### CrawlLog
- Crawler-Runs tracking
- Statistiken (new/updated/failed)
- Error Messages
- Duration

## Tests

```bash
# Alle Tests ausführen
pytest

# Mit Coverage
pytest --cov=src tests/

# Einzelne Testdatei
pytest tests/test_chunker.py -v
```

## Entwicklung

```bash
# Code Formatierung
black src/ tests/

# Linting
ruff check src/ tests/

# Type Checking
mypy src/
```

## Performance

- **Bundesrecht**: ~5.000 Dokumente in ~2-3h
- **Bayern.Recht**: ~1.000 Dokumente in ~1-2h (Rate Limit!)
- **EUR-Lex**: ~100 Dokumente in ~30min

Bottleneck: Embedding Generation (GPU empfohlen!)

## Troubleshooting

### Out of Memory bei Embeddings
```python
# In config.py reduzieren:
EMBEDDING_BATCH_SIZE=4  # statt 8
```

### Encoding Fehler
```python
# Parser versucht automatisch UTF-8 und ISO-8859-1
# Bei Problemen: encoding='latin1' verwenden
```

### Rate Limiting
```bash
# Crawler_RATE_LIMIT erhöhen (Vorsicht bei externen Sites!)
CRAWLER_RATE_LIMIT=0.5  # 2 req/sec
```

## Lizenz

MIT

## Hinweise

⚠️ **Respektiere robots.txt und Rate Limits!**

Die gecrawlten Rechtstexte sind gemäß § 5 Abs. 1 UrhG gemeinfrei.
