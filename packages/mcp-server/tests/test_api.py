"""Tests for API routes."""

import pytest
from httpx import AsyncClient

from src.main import app


@pytest.mark.asyncio
async def test_root_endpoint():
    """Test root endpoint."""
    async with AsyncClient(app=app, base_url="http://test") as client:
        response = await client.get("/")

    assert response.status_code == 200
    data = response.json()
    assert "service" in data
    assert "version" in data


@pytest.mark.asyncio
async def test_health_check():
    """Test health check endpoint."""
    async with AsyncClient(app=app, base_url="http://test") as client:
        response = await client.get("/api/health")

    assert response.status_code == 200
    data = response.json()
    assert "status" in data
    assert "database" in data
    assert "model" in data


@pytest.mark.asyncio
async def test_search_laws_endpoint():
    """Test search laws endpoint."""
    async with AsyncClient(app=app, base_url="http://test") as client:
        response = await client.post(
            "/api/search",
            json={
                "query": "Datenschutz",
                "limit": 5,
            },
        )

    assert response.status_code == 200
    data = response.json()
    assert "results" in data
    assert "total" in data
