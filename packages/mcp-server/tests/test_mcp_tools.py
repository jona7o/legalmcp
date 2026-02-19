"""Tests for MCP tools."""

import pytest
from uuid import uuid4

from src.mcp.tools import MCPTools


@pytest.mark.asyncio
async def test_search_laws(db_session):
    """Test law search functionality."""
    tools = MCPTools(db_session)

    result = await tools.search_laws(
        query="Datenschutz",
        limit=5,
    )

    assert "results" in result
    assert "total" in result
    assert isinstance(result["results"], list)


@pytest.mark.asyncio
async def test_get_law_by_abbreviation(db_session):
    """Test getting law by abbreviation."""
    tools = MCPTools(db_session)

    result = await tools.get_law_by_id(abbreviation="BGB")

    # This will be None unless data exists
    # In production, mock the database or use test fixtures
    assert result is None or isinstance(result, dict)


@pytest.mark.asyncio
async def test_search_case_law(db_session):
    """Test case law search."""
    tools = MCPTools(db_session)

    result = await tools.search_case_law(
        query="Mietrecht",
        limit=5,
    )

    assert "results" in result
    assert "total" in result


@pytest.mark.asyncio
async def test_get_legal_changes(db_session):
    """Test legal changes retrieval."""
    tools = MCPTools(db_session)

    result = await tools.get_legal_changes(days=30)

    assert "changes" in result
    assert "total" in result
    assert "days" in result


@pytest.mark.asyncio
async def test_get_related_laws(db_session):
    """Test related laws retrieval."""
    tools = MCPTools(db_session)

    # Use a random UUID for testing (will not exist)
    result = await tools.get_related_laws(law_id=uuid4())

    assert "error" in result or "related_laws" in result
