#!/bin/bash

# Legal MCP Server - Quick Start Script

set -e

echo "🚀 Legal MCP Server - Quick Start"
echo "=================================="
echo ""

# Check Python version
echo "📋 Checking Python version..."
python_version=$(python3 --version 2>&1 | awk '{print $2}')
echo "Found Python $python_version"

# Check if PostgreSQL is running
echo ""
echo "📋 Checking PostgreSQL..."
if command -v docker &> /dev/null; then
    echo "Docker found. Starting PostgreSQL with docker-compose..."
    docker-compose up -d postgres
    echo "Waiting for PostgreSQL to be ready..."
    sleep 10
else
    echo "⚠️  Docker not found. Please ensure PostgreSQL is running manually."
fi

# Create virtual environment
echo ""
echo "📦 Creating virtual environment..."
if [ ! -d "venv" ]; then
    python3 -m venv venv
    echo "Virtual environment created."
else
    echo "Virtual environment already exists."
fi

# Activate virtual environment
echo ""
echo "🔌 Activating virtual environment..."
source venv/bin/activate

# Install dependencies
echo ""
echo "📚 Installing dependencies..."
pip install -q --upgrade pip
pip install -q -r requirements.txt
echo "Dependencies installed."

# Setup environment
echo ""
echo "⚙️  Setting up environment..."
if [ ! -f ".env" ]; then
    cp .env.example .env
    echo "Environment file created (.env)"
else
    echo "Environment file already exists."
fi

# Download ML model
echo ""
echo "🤖 Downloading ML model (this may take a while)..."
python3 -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('deepset/gbert-base')" || echo "Model download skipped."

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Start the server:"
echo "   uvicorn src.main:app --reload --port 8000"
echo ""
echo "2. Or use the Makefile:"
echo "   make dev"
echo ""
echo "3. Access the API docs:"
echo "   http://localhost:8000/docs"
echo ""
echo "4. Test the API:"
echo "   curl -H 'X-API-Key: dev_key_12345' http://localhost:8000/api/health"
echo ""
