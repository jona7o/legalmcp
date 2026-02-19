.PHONY: help install dev test lint format clean run docker-build docker-run

help:
	@echo "Legal MCP Server - Makefile Commands"
	@echo "====================================="
	@echo "install      - Install dependencies"
	@echo "dev          - Run development server"
	@echo "test         - Run tests"
	@echo "lint         - Run linters"
	@echo "format       - Format code"
	@echo "clean        - Clean temporary files"
	@echo "run          - Run production server"
	@echo "docker-build - Build Docker image"
	@echo "docker-run   - Run Docker container"

install:
	pip install -r requirements.txt

dev:
	uvicorn src.main:app --reload --port 8000 --log-level debug

test:
	pytest tests/ -v --cov=src --cov-report=html

lint:
	ruff src/ tests/
	mypy src/

format:
	black src/ tests/
	ruff --fix src/ tests/

clean:
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -type d -name .pytest_cache -exec rm -rf {} +
	find . -type d -name .mypy_cache -exec rm -rf {} +
	find . -type d -name .ruff_cache -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete
	rm -rf htmlcov/

run:
	uvicorn src.main:app --host 0.0.0.0 --port 8000 --workers 4

docker-build:
	docker build -t legalmcp-server:latest .

docker-run:
	docker run -p 8000:8000 \
		-e POSTGRES_HOST=host.docker.internal \
		-e POSTGRES_PASSWORD=legal_dev_password \
		legalmcp-server:latest

mcp-stdio:
	python -m src.mcp.protocol

db-init:
	docker-compose up -d postgres
	sleep 5
	@echo "Database initialized with schema from migrations/init.sql"
