# Legal MCP - Testing Guide

## 🎯 Quick Start

```bash
# 1. Stelle sicher, dass Docker läuft
docker info

# 2. Umgebungsvariablen prüfen
cat .env

# 3. Alle Services starten
docker-compose up -d

# 4. Logs beobachten
docker-compose logs -f

# 5. Services testen
curl http://localhost:8000/api/health  # Backend
curl http://localhost:3000              # Frontend
```

---

## 📋 Schritt-für-Schritt Test

### 1. Vorbereitung

```bash
cd /Users/tobias.jonas/Developer/opensource/legalmcp

# Sicherstellen dass .env existiert
ls -la .env

# Docker Compose Version prüfen
docker-compose --version
```

### 2. Database Build & Start

```bash
# Postgres als erstes starten (mit pgvector extension)
docker-compose up -d postgres

# Warte auf DB Ready (max 30 Sekunden)
docker-compose logs postgres | grep "database system is ready"

# Prüfe pgvector Extension
docker-compose exec postgres psql -U legal -d legalmcp -c "SELECT * FROM pg_extension WHERE extname='vector';"
```

**Erwartetes Ergebnis**:
```
 oid  | extname | extowner | extnamespace | extrelocatable | extversion | extconfig | extcondition 
------+---------+----------+--------------+----------------+------------+-----------+--------------
 16386| vector  |       10 |         2200 | t              | 0.8.0      |           | 
```

### 3. MCP Server Build & Start

```bash
# Build Image (kann 5-10 Minuten dauern wegen ML Models)
docker-compose build mcp-server

# Starten
docker-compose up -d mcp-server

# Logs prüfen
docker-compose logs -f mcp-server
```

**Erwartete Log-Ausgaben**:
```
INFO:     Started server process
INFO:     Waiting for application startup.
INFO:     Application startup complete.
INFO:     Uvicorn running on http://0.0.0.0:8000
```

**Health Check**:
```bash
curl http://localhost:8000/api/health
```

**Erwartete Antwort**:
```json
{
  "status": "healthy",
  "database": "connected",
  "embedding_model": "loaded"
}
```

### 4. Frontend Build & Start

```bash
# Build Image (kann 3-5 Minuten dauern)
docker-compose build frontend

# Starten
docker-compose up -d frontend

# Logs prüfen
docker-compose logs -f frontend
```

**Erwartete Log-Ausgaben**:
```
ready - started server on 0.0.0.0:3000, url: http://localhost:3000
event - compiled client and server successfully
```

**Browser Test**:
- Öffne: http://localhost:3000
- Erwartung: Legal MCP Startseite mit Suchfeld

### 5. Datenbank Schema Prüfung

```bash
# Alle Tabellen anzeigen
docker-compose exec postgres psql -U legal -d legalmcp -c "\dt"
```

**Erwartete Tabellen**:
```
              List of relations
 Schema |      Name       | Type  | Owner 
--------+-----------------+-------+-------
 public | api_keys        | table | legal
 public | case_law        | table | legal
 public | case_law_chunks | table | legal
 public | crawl_runs      | table | legal
 public | law_chunks      | table | legal
 public | laws            | table | legal
 public | related_laws    | table | legal
```

**Prüfe pgvector Indizes**:
```bash
docker-compose exec postgres psql -U legal -d legalmcp -c "
  SELECT indexname, tablename 
  FROM pg_indexes 
  WHERE tablename IN ('law_chunks', 'case_law_chunks');
"
```

### 6. Crawler Test (Optional - Initial Data Seed)

```bash
# Crawler einmalig ausführen
docker-compose --profile crawler run --rm crawler

# Oder im Hintergrund:
docker-compose --profile crawler up -d crawler

# Logs folgen
docker-compose logs -f crawler
```

**Erwartete Ausgaben**:
```
INFO - Starting crawler orchestrator
INFO - Bundesrecht Crawler: Starting...
INFO - Found 100 laws to crawl
INFO - Processing: Bürgerliches Gesetzbuch (BGB)
INFO - Created law: BGB (UUID: ...)
INFO - Generated 543 chunks for BGB
```

**Nach Crawler-Lauf prüfen**:
```bash
# Anzahl gecrawlter Gesetze
docker-compose exec postgres psql -U legal -d legalmcp -c "SELECT COUNT(*) FROM laws;"

# Anzahl Chunks
docker-compose exec postgres psql -U legal -d legalmcp -c "SELECT COUNT(*) FROM law_chunks;"

# Chunks mit Embeddings
docker-compose exec postgres psql -U legal -d legalmcp -c "
  SELECT COUNT(*) 
  FROM law_chunks 
  WHERE embedding IS NOT NULL;
"
```

### 7. API Tests

#### Search Test (POST /api/search)
```bash
curl -X POST http://localhost:8000/api/search \
  -H "Content-Type: application/json" \
  -H "X-API-Key: dev_key_12345" \
  -d '{
    "query": "Datenschutz",
    "limit": 5,
    "min_score": 0.7
  }' | jq
```

**Erwartete Antwort** (falls Daten vorhanden):
```json
{
  "results": [
    {
      "law_id": "uuid-here",
      "title": "Datenschutz-Grundverordnung",
      "abbreviation": "DSGVO",
      "chunk_content": "...",
      "score": 0.89,
      "jurisdiction": "eu"
    }
  ]
}
```

#### Get Law Test
```bash
# Ersetze UUID mit einer echten aus der DB
LAW_ID=$(docker-compose exec -T postgres psql -U legal -d legalmcp -tAc "SELECT id FROM laws LIMIT 1;")
curl -H "X-API-Key: dev_key_12345" http://localhost:8000/api/laws/$LAW_ID | jq
```

### 8. Frontend Integration Test

1. **Öffne Browser**: http://localhost:3000
2. **Suchfeld Test**:
   - Eingabe: "Datenschutz"
   - Klicke "Suchen"
   - Erwartung: Ergebnisliste (falls Daten vorhanden)

3. **Filter Test**:
   - Wähle "EU" Checkbox
   - Erwartung: Nur EU-Recht angezeigt

4. **Detail-Ansicht Test**:
   - Klicke auf ein Suchergebnis
   - Erwartung: Gesetzestext mit Paragraphen

---

## 🔍 Troubleshooting

### Problem: Postgres startet nicht

```bash
# Logs prüfen
docker-compose logs postgres

# Port bereits belegt?
lsof -i :5432

# Volumes löschen und neu starten
docker-compose down -v
docker-compose up -d postgres
```

### Problem: MCP Server kann nicht zur DB connecten

```bash
# Environment prüfen
docker-compose exec mcp-server env | grep POSTGRES

# DB Connection manuell testen
docker-compose exec mcp-server python -c "
import asyncpg
import asyncio
async def test():
    conn = await asyncpg.connect(
        host='postgres',
        port=5432,
        user='legal',
        password='legal_dev_password',
        database='legalmcp'
    )
    print('Connected!')
    await conn.close()
asyncio.run(test())
"
```

### Problem: Frontend kann Backend nicht erreichen

```bash
# Netzwerk prüfen
docker network inspect legalmcp_legalmcp-network

# DNS Resolution testen
docker-compose exec frontend ping mcp-server

# Environment prüfen
docker-compose exec frontend env | grep NEXT_PUBLIC
```

### Problem: Embeddings werden nicht generiert

```bash
# Model Download Status prüfen
docker-compose exec mcp-server ls -lh /app/models

# Python Sentence Transformers Test
docker-compose exec mcp-server python -c "
from sentence_transformers import SentenceTransformer
model = SentenceTransformer('deepset/gbert-base')
print('Model loaded successfully!')
print(f'Embedding size: {model.get_sentence_embedding_dimension()}')
"
```

### Problem: Docker Build schlägt fehl

```bash
# Cache löschen und neu bauen
docker-compose build --no-cache mcp-server

# Für einzelne Services
docker-compose build --no-cache frontend

# Alle Services gleichzeitig (dauert lang!)
docker-compose build --parallel
```

---

## 📊 Monitoring

### Container Status

```bash
docker-compose ps
```

**Erwartete Ausgabe**:
```
NAME                  STATUS    PORTS
legalmcp-postgres     Up        0.0.0.0:5432->5432/tcp
legalmcp-mcp-server   Up        0.0.0.0:8000->8000/tcp
legalmcp-frontend     Up        0.0.0.0:3000->3000/tcp
```

### Resource Usage

```bash
docker stats
```

### Logs

```bash
# Alle Services
docker-compose logs -f

# Einzelner Service
docker-compose logs -f mcp-server

# Letzte 100 Zeilen
docker-compose logs --tail=100 mcp-server
```

---

## 🧹 Cleanup

### Services stoppen

```bash
# Alle Services stoppen
docker-compose down

# Mit Volume-Löschung (ACHTUNG: Daten gehen verloren!)
docker-compose down -v
```

### Komplett aufräumen

```bash
# Services, Volumes, Networks löschen
docker-compose down -v --remove-orphans

# Zusätzlich: Images löschen
docker images | grep legalmcp | awk '{print $3}' | xargs docker rmi

# Docker System Prune (optional, vorsicht!)
docker system prune -a --volumes
```

---

## ✅ Success Criteria

Das System ist erfolgreich, wenn:

1. ✅ Alle 3 Container laufen (`docker-compose ps`)
2. ✅ Backend Health Check antwortet (`curl localhost:8000/api/health`)
3. ✅ Frontend lädt (`curl localhost:3000`)
4. ✅ Datenbank hat korrekte Tabellen (`\dt` zeigt 7 Tabellen)
5. ✅ pgvector Extension ist aktiv
6. ✅ Search API funktioniert (POST /api/search)
7. ✅ Frontend kann Backend erreichen (keine CORS Errors)

---

## 📞 Support

Bei Problemen:
1. Prüfe Logs: `docker-compose logs -f`
2. Prüfe `QM_FIXES_APPLIED.md` für bekannte Issues
3. Prüfe GitHub Issues: https://github.com/yourorg/legalmcp/issues
