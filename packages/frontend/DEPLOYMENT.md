# Legal MCP - Deployment Guide

## Schnellstart

### Lokale Entwicklung

1. **Dependencies installieren**
   ```bash
   cd packages/frontend
   pnpm install
   ```

2. **Umgebungsvariablen konfigurieren**
   ```bash
   cp .env.example .env.local
   ```

3. **Dev Server starten**
   ```bash
   pnpm dev
   ```
   
   Das Frontend läuft auf http://localhost:3000

### Docker Deployment

#### Gesamtes System starten
```bash
# Alle Services starten
docker-compose up -d

# Logs verfolgen
docker-compose logs -f frontend

# Services stoppen
docker-compose down
```

#### Nur Frontend bauen
```bash
cd packages/frontend
docker build -t legalmcp-frontend .
docker run -p 3000:3000 \
  -e NEXT_PUBLIC_API_URL=http://localhost:8000 \
  legalmcp-frontend
```

## Produktions-Deployment

### 1. Environment Variables

Für Produktion in `.env.production`:

```env
NEXT_PUBLIC_API_URL=https://api.your-domain.com
```

### 2. Build

```bash
pnpm build
```

### 3. Start

```bash
pnpm start
```

### 4. Docker Production

```bash
docker build -t legalmcp-frontend:latest .
docker run -d \
  --name legalmcp-frontend \
  -p 3000:3000 \
  -e NEXT_PUBLIC_API_URL=https://api.your-domain.com \
  --restart unless-stopped \
  legalmcp-frontend:latest
```

## Vercel Deployment

1. **Projekt mit Vercel verbinden**
   ```bash
   npx vercel
   ```

2. **Environment Variables in Vercel setzen**
   - `NEXT_PUBLIC_API_URL`: URL des Backend-APIs

3. **Deploy**
   ```bash
   npx vercel --prod
   ```

## Kubernetes Deployment

Beispiel `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: legalmcp-frontend
spec:
  replicas: 2
  selector:
    matchLabels:
      app: legalmcp-frontend
  template:
    metadata:
      labels:
        app: legalmcp-frontend
    spec:
      containers:
      - name: frontend
        image: legalmcp-frontend:latest
        ports:
        - containerPort: 3000
        env:
        - name: NEXT_PUBLIC_API_URL
          value: "http://legalmcp-backend:8000"
---
apiVersion: v1
kind: Service
metadata:
  name: legalmcp-frontend
spec:
  selector:
    app: legalmcp-frontend
  ports:
  - port: 80
    targetPort: 3000
  type: LoadBalancer
```

## Performance Optimierung

### 1. CDN Integration
- Statische Assets über CDN ausliefern
- Next.js Image Optimization nutzen

### 2. Caching
- Browser Caching für Assets aktivieren
- API Responses cachen (SWR/React Query)

### 3. Code Splitting
- Automatisch durch Next.js
- Lazy Loading für große Komponenten

## Monitoring

### Health Check
```bash
curl http://localhost:3000/api/health
```

### Logs
```bash
# Docker Logs
docker logs legalmcp-frontend

# Docker Compose
docker-compose logs -f frontend
```

## Troubleshooting

### Port bereits belegt
```bash
# Port ändern
PORT=3001 pnpm dev
```

### Build Fehler
```bash
# Cache löschen
rm -rf .next
pnpm build
```

### API Verbindungsprobleme
- NEXT_PUBLIC_API_URL prüfen
- CORS-Einstellungen im Backend prüfen
- Network-Konfiguration in Docker prüfen

## Sicherheit

1. **Environment Variables**
   - Niemals Secrets im Code
   - `.env` Dateien nicht committen

2. **CSP Headers**
   - Content Security Policy konfigurieren
   - XSS Protection aktivieren

3. **HTTPS**
   - In Produktion nur über HTTPS
   - Reverse Proxy (nginx/traefik) empfohlen

## Updates

```bash
# Dependencies aktualisieren
pnpm update

# Next.js aktualisieren
pnpm add next@latest react@latest react-dom@latest

# Type Definitions aktualisieren
pnpm add -D @types/react@latest @types/react-dom@latest
```
