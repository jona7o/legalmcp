# Legal MCP Frontend

Next.js 14 Frontend mit shadcn/ui für die semantische Suche in deutschem und EU-Recht.

## Features

- 🔍 Semantische Suche über deutsche Gesetze, bayerisches Landesrecht und EU-Verordnungen
- ⚖️ Detailansichten für Gesetze und Rechtsprechung
- 🔄 Übersicht aktueller Gesetzesänderungen
- 🎨 Modernes UI mit shadcn/ui und Tailwind CSS
- 🌙 Dark Mode Support
- 📱 Responsive Design

## Entwicklung

```bash
# Dependencies installieren
pnpm install

# Dev Server starten (Port 3000)
pnpm dev

# Production Build
pnpm build

# Production Server
pnpm start
```

## Umgebungsvariablen

Erstellen Sie eine `.env.local` Datei:

```env
NEXT_PUBLIC_API_URL=http://localhost:8000
```

## Seitenstruktur

- `/` - Hauptsuche mit Filtern
- `/law/[id]` - Gesetzesdetailansicht
- `/case/[id]` - Urteilsansicht
- `/changes` - Aktuelle Änderungen

## Docker

```bash
# Image bauen
docker build -t legalmcp-frontend .

# Container starten
docker run -p 3000:3000 -e NEXT_PUBLIC_API_URL=http://backend:8000 legalmcp-frontend
```

## Rechtlicher Hinweis

Die Informationen auf dieser Plattform dienen ausschließlich zu Informationszwecken und stellen keine Rechtsberatung dar.
