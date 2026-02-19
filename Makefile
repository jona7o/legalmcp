.PHONY: help dev up down logs build test clean install crawler-run db-migrate

help: ## Zeigt diese Hilfe an
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

install: ## Installiert alle Dependencies
	@echo "📦 Installiere Backend Dependencies..."
	cd packages/mcp-server && pip install -r requirements.txt
	cd packages/crawler && pip install -r requirements.txt
	@echo "📦 Installiere Frontend Dependencies..."
	cd packages/frontend && pnpm install
	@echo "✅ Installation abgeschlossen!"

dev: ## Startet alle Services im Development Mode
	docker-compose up -d

up: dev ## Alias für dev

down: ## Stoppt alle Services
	docker-compose down

logs: ## Zeigt Logs aller Services
	docker-compose logs -f

build: ## Baut alle Docker Images neu
	docker-compose build

clean: ## Räumt auf (stoppt Container, löscht Volumes)
	docker-compose down -v
	rm -rf packages/*/venv
	rm -rf packages/frontend/node_modules
	rm -rf packages/frontend/.next

test: ## Führt alle Tests aus
	@echo "🧪 Teste Crawler..."
	cd packages/crawler && pytest tests/ -v
	@echo "🧪 Teste MCP Server..."
	cd packages/mcp-server && pytest tests/ -v
	@echo "🧪 Teste Frontend..."
	cd packages/frontend && pnpm test
	@echo "✅ Alle Tests erfolgreich!"

lint: ## Führt Linting für alle Packages aus
	@echo "🔍 Linting Crawler..."
	cd packages/crawler && ruff check src/
	@echo "🔍 Linting MCP Server..."
	cd packages/mcp-server && ruff check src/
	@echo "🔍 Linting Frontend..."
	cd packages/frontend && pnpm lint
	@echo "✅ Linting abgeschlossen!"

crawler-run: ## Führt den Crawler manuell aus
	docker-compose run --rm crawler

db-migrate: ## Führt Datenbank-Migrationen aus
	docker-compose exec postgres psql -U legal -d legalmcp -f /docker-entrypoint-initdb.d/init.sql

db-shell: ## Öffnet eine PostgreSQL Shell
	docker-compose exec postgres psql -U legal -d legalmcp

db-reset: ## Setzt die Datenbank zurück (ACHTUNG: Löscht alle Daten!)
	@read -p "⚠️  Wirklich alle Daten löschen? [y/N] " confirm; \
	if [ "$$confirm" = "y" ]; then \
		docker-compose down -v && \
		docker-compose up -d postgres && \
		sleep 5 && \
		make db-migrate; \
	fi

ps: ## Zeigt Status aller Services
	docker-compose ps

frontend-dev: ## Startet nur das Frontend im Dev-Mode
	cd packages/frontend && pnpm dev

backend-dev: ## Startet nur das Backend im Dev-Mode
	cd packages/mcp-server && uvicorn src.main:app --reload

health: ## Prüft Health Status aller Services
	@echo "🏥 Prüfe Backend..."
	@curl -s http://localhost:8000/api/health | jq . || echo "❌ Backend nicht erreichbar"
	@echo "🏥 Prüfe Frontend..."
	@curl -s http://localhost:3000 > /dev/null && echo "✅ Frontend läuft" || echo "❌ Frontend nicht erreichbar"
	@echo "🏥 Prüfe Datenbank..."
	@docker-compose exec -T postgres pg_isready -U legal && echo "✅ Datenbank läuft" || echo "❌ Datenbank nicht erreichbar"

stats: ## Zeigt Statistiken des Projekts
	@echo "📊 Projekt Statistiken:"
	@echo ""
	@echo "🐍 Python Code:"
	@find packages/crawler/src packages/mcp-server/src -name "*.py" | xargs wc -l | tail -1
	@echo ""
	@echo "🎨 TypeScript Code:"
	@find packages/frontend/src -name "*.ts" -o -name "*.tsx" | xargs wc -l | tail -1
	@echo ""
	@echo "📁 Dateien:"
	@find packages -type f | wc -l
	@echo ""
	@echo "📦 Packages:"
	@ls -d packages/*/ | wc -l

init: ## Initialisiert das Projekt (erste Einrichtung)
	@echo "🚀 Initialisiere Legal MCP..."
	cp .env.example .env
	@echo "✅ .env erstellt - bitte anpassen!"
	@echo "📦 Installiere Dependencies..."
	make install
	@echo "🐳 Starte Docker Services..."
	make up
	@echo ""
	@echo "✅ Initialisierung abgeschlossen!"
	@echo ""
	@echo "Nächste Schritte:"
	@echo "  1. Passe .env an deine Umgebung an"
	@echo "  2. Führe 'make crawler-run' aus für initialen Daten-Import"
	@echo "  3. Öffne http://localhost:3000 für das Frontend"
	@echo "  4. API: http://localhost:8000/docs"
