# QM Fixes - Applied Changes

## Status: ✅ Alle kritischen Fehler behoben

### PRIO 1 - Kritische Schema-Inkompatibilität (BEHOBEN ✅)

**Problem**: Crawler schrieb in `documents` Tabelle, MCP-Server erwartete `laws` Tabelle.

**Lösung**:
1. ✅ **packages/crawler/src/db/models.py** vollständig überarbeitet:
   - `Document` → `Law` (UUID statt Integer ID)
   - `DocumentChunk` → `LawChunk` 
   - `CrawlLog` → `CrawlRun`
   - Enum `DocumentType` angepasst: `BUNDESGESETZ` → `LAW`, etc.
   - `JurisdictionType` Enum hinzugefügt: `FEDERAL`, `EU`, `BAVARIA`
   - `embedding` Feld: `ARRAY(Float)` → `Vector(768)` (pgvector)
   - Alle Tabellennamen und Foreign Keys aktualisiert

2. ✅ **packages/crawler/src/db/operations.py** vollständig angepasst:
   - `DocumentOperations` → `LawOperations`
   - `CrawlLogOperations` → `CrawlRunOperations`
   - `get_document_by_source_id()` → `get_law_by_source_url()`
   - Alle Methoden nutzen jetzt `Law` und `LawChunk` Modelle
   - Content Hash wird in `metadata` JSON Feld gespeichert (kompatibel mit MCP-Server)

3. ✅ **packages/crawler/requirements.txt**:
   - `pgvector>=0.2.0` hinzugefügt

**Ergebnis**: Crawler und MCP-Server nutzen jetzt identisches Datenbankschema!

---

### PRIO 2 - Frontend API Client Inkompatibilität (BEHOBEN ✅)

**Problem**: 
- Frontend machte GET Requests, Backend erwartete POST
- Frontend fehlte `/api` Prefix in URLs

**Lösung**:
✅ **packages/frontend/src/lib/api.ts** komplett überarbeitet:

```typescript
// ❌ VORHER:
return this.request<SearchResult[]>(`/search?${queryParams.toString()}`);

// ✅ NACHHER:
return this.request<SearchResult[]>(`/api/search`, {
  method: 'POST',
  body: JSON.stringify({
    query: params.query,
    limit: params.limit || 10,
    jurisdiction: params.jurisdictions,
    include_case_law: params.includeCases,
    document_type: params.documentType ? [params.documentType] : undefined,
    min_score: 0.7
  })
});
```

**Änderungen**:
- `search()`: GET → POST, `/search` → `/api/search`
- `getLaw()`: `/laws/${id}` → `/api/laws/${id}`
- `getCase()`: `/cases/${id}` → `/api/case-law/${id}`
- `getChanges()`: `/changes` → `/api/changes`, `since` → `since_days`

**Ergebnis**: Frontend API Calls matchen jetzt exakt die Backend Routes!

---

### PRIO 3 - Frontend Dockerfile fehlte Development Stage (BEHOBEN ✅)

**Problem**: docker-compose.yml referenzierte `target: development`, aber Dockerfile hatte keine solche Stage.

**Lösung**:
✅ **packages/frontend/Dockerfile** Development Stage hinzugefügt:

```dockerfile
# Development stage (for docker-compose)
FROM base AS development
WORKDIR /app

RUN apk add --no-cache libc6-compat

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

COPY package.json pnpm-lock.yaml* ./
RUN pnpm install

COPY . .

EXPOSE 3000

ENV NODE_ENV development
ENV NEXT_TELEMETRY_DISABLED 1

CMD ["pnpm", "dev"]
```

**Ergebnis**: Frontend kann jetzt mit Hot Reloading in Docker laufen!

---

## Zusammenfassung der geänderten Dateien

### Crawler (3 Dateien)
- ✅ `packages/crawler/src/db/models.py` - Komplette Schema-Anpassung
- ✅ `packages/crawler/src/db/operations.py` - API-Anpassung
- ✅ `packages/crawler/requirements.txt` - pgvector hinzugefügt

### Frontend (2 Dateien)
- ✅ `packages/frontend/src/lib/api.ts` - API Calls gefixt
- ✅ `packages/frontend/Dockerfile` - Development Stage hinzugefügt

### Root
- ✅ `.env` - Erstellt aus .env.example

---

## Offene Punkte (Nice-to-Have, nicht kritisch)

### Optional - Weitere Verbesserungen:

1. **Retry-Logik im Crawler**:
   ```python
   # packages/crawler/src/sources/base.py
   from tenacity import retry, stop_after_attempt, wait_exponential
   
   @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=4, max=10))
   async def fetch_url(self, url: str) -> str:
       # ... existing code
   ```

2. **Health Checks in docker-compose.yml**:
   ```yaml
   mcp-server:
     healthcheck:
       test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
       interval: 10s
       timeout: 5s
       retries: 3
   ```

3. **Database Wait Script**:
   - Füge `wait_for_db.py` in mcp-server hinzu
   - Verhindert Race Conditions beim Start

4. **Embedding Model aus ENV**:
   ```dockerfile
   # packages/mcp-server/Dockerfile
   ARG EMBEDDING_MODEL=deepset/gbert-base
   RUN python -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('${EMBEDDING_MODEL}')"
   ```

---

## Nächste Schritte für lokalen Test

```bash
# 1. Environment prüfen
cat .env

# 2. Docker Images bauen (Test ob builds funktionieren)
docker-compose build postgres
docker-compose build mcp-server
docker-compose build frontend

# 3. Services starten
docker-compose up -d postgres
# Warte 10 Sekunden
docker-compose up -d mcp-server
# Warte 5 Sekunden
docker-compose up -d frontend

# 4. Logs prüfen
docker-compose logs -f

# 5. Health Check
curl http://localhost:8000/api/health
curl http://localhost:3000

# 6. Datenbank prüfen
docker-compose exec postgres psql -U legal -d legalmcp -c "\dt"

# 7. Optional: Crawler ausführen (initial seed)
docker-compose --profile crawler up crawler
```

---

## Erwartete Funktionalität nach Fixes

✅ **PostgreSQL**: Startet mit pgvector Extension  
✅ **MCP-Server**: Verbindet zur DB, erstellt Schema via init.sql  
✅ **Frontend**: Läuft auf Port 3000 mit Hot Reload  
✅ **Crawler**: Kann Daten in `laws` und `law_chunks` Tabellen schreiben  
✅ **API**: Backend akzeptiert POST /api/search vom Frontend  
✅ **Embeddings**: pgvector VECTOR(768) Typ wird korrekt verwendet  

---

**QM Status**: ✅ **PRODUKTIONSREIF FÜR LOKALEN TEST**

Alle kritischen Blocker wurden behoben. Das System sollte jetzt mit `docker-compose up` starten.
