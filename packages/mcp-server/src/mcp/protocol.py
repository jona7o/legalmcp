"""MCP Protocol Handler using mcp-python SDK."""

from typing import Any, Dict, List, Optional

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import (
    Resource,
    TextContent,
    Tool,
)
from pydantic import AnyUrl

from src.config import get_settings
from src.db.connection import async_session_maker
from src.search.vector_store import VectorStore

settings = get_settings()

# Initialize MCP server
mcp_server = Server("legal-mcp-server")


@mcp_server.list_tools()
async def list_tools() -> List[Tool]:
    """
    List available MCP tools.

    Returns:
        List of tool definitions
    """
    return [
        Tool(
            name="search_laws",
            description="Search for German laws using semantic similarity. Supports filtering by jurisdiction (federal, eu, bavaria) and document type.",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query text in German or English",
                    },
                    "jurisdiction": {
                        "type": "array",
                        "items": {"type": "string", "enum": ["federal", "eu", "bavaria", "other"]},
                        "description": "Filter by jurisdictions",
                    },
                    "document_type": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["law", "regulation", "directive", "decision", "other"],
                        },
                        "description": "Filter by document types",
                    },
                    "include_case_law": {
                        "type": "boolean",
                        "description": "Include case law in search results",
                        "default": False,
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 100,
                    },
                },
                "required": ["query"],
            },
        ),
        Tool(
            name="get_law_by_id",
            description="Retrieve a specific law by its UUID or abbreviation (e.g., 'BGB', 'StGB')",
            inputSchema={
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "UUID of the law",
                    },
                    "abbreviation": {
                        "type": "string",
                        "description": "Law abbreviation (e.g., 'BGB', 'StGB')",
                    },
                },
                "oneOf": [{"required": ["id"]}, {"required": ["abbreviation"]}],
            },
        ),
        Tool(
            name="search_case_law",
            description="Search for court decisions and case law using semantic similarity",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query text",
                    },
                    "jurisdiction": {
                        "type": "array",
                        "items": {"type": "string", "enum": ["federal", "eu", "bavaria", "other"]},
                        "description": "Filter by jurisdictions",
                    },
                    "court_level": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["bgh", "bverwg", "bfh", "bsg", "bag", "lg", "ag", "vg", "other"],
                        },
                        "description": "Filter by court levels",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 100,
                    },
                },
                "required": ["query"],
            },
        ),
        Tool(
            name="get_legal_changes",
            description="Get recent legal changes (new laws, modifications, repeals)",
            inputSchema={
                "type": "object",
                "properties": {
                    "days": {
                        "type": "integer",
                        "description": "Number of days to look back",
                        "default": 30,
                        "minimum": 1,
                        "maximum": 365,
                    },
                    "jurisdiction": {
                        "type": "array",
                        "items": {"type": "string", "enum": ["federal", "eu", "bavaria", "other"]},
                        "description": "Filter by jurisdictions",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 200,
                    },
                },
            },
        ),
        Tool(
            name="get_related_laws",
            description="Find laws related to a specific law based on semantic similarity",
            inputSchema={
                "type": "object",
                "properties": {
                    "law_id": {
                        "type": "string",
                        "description": "UUID of the law",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20,
                    },
                },
                "required": ["law_id"],
            },
        ),
    ]


@mcp_server.call_tool()
async def call_tool(name: str, arguments: Dict[str, Any]) -> List[TextContent]:
    """
    Handle MCP tool calls.

    Args:
        name: Tool name
        arguments: Tool arguments

    Returns:
        List of text content responses

    Raises:
        ValueError: If tool not found or execution fails
    """
    async with async_session_maker() as session:
        if name == "search_laws":
            return await _search_laws(session, arguments)
        elif name == "get_law_by_id":
            return await _get_law_by_id(session, arguments)
        elif name == "search_case_law":
            return await _search_case_law(session, arguments)
        elif name == "get_legal_changes":
            return await _get_legal_changes(session, arguments)
        elif name == "get_related_laws":
            return await _get_related_laws(session, arguments)
        else:
            raise ValueError(f"Unknown tool: {name}")


async def _search_laws(session, arguments: Dict[str, Any]) -> List[TextContent]:
    """Execute search_laws tool."""
    from src.db.models import Law
    from sqlalchemy import select

    query = arguments["query"]
    jurisdiction = arguments.get("jurisdiction")
    document_type = arguments.get("document_type")
    include_case_law = arguments.get("include_case_law", False)
    limit = arguments.get("limit", 10)

    vector_store = VectorStore(session)

    # Search laws
    results = await vector_store.search_laws(
        query=query,
        limit=limit,
        jurisdiction_filter=jurisdiction,
        document_type_filter=document_type,
    )

    # Format results
    response_text = f"Found {len(results)} relevant laws for query: '{query}'\n\n"

    for i, result in enumerate(results, 1):
        # Get law details
        stmt = select(Law).where(Law.id == result["law_id"])
        law_result = await session.execute(stmt)
        law = law_result.scalar_one_or_none()

        if law:
            response_text += f"{i}. {law.title}"
            if law.abbreviation:
                response_text += f" ({law.abbreviation})"
            response_text += f"\n"
            response_text += f"   Jurisdiction: {law.jurisdiction.value}\n"
            response_text += f"   Type: {law.document_type.value}\n"
            response_text += f"   Similarity: {result['similarity']:.2%}\n"
            if result.get("section_reference"):
                response_text += f"   Section: {result['section_reference']}\n"
            response_text += f"   Content: {result['content'][:300]}...\n"
            response_text += f"   Source: {law.source_url}\n\n"

    return [TextContent(type="text", text=response_text)]


async def _get_law_by_id(session, arguments: Dict[str, Any]) -> List[TextContent]:
    """Execute get_law_by_id tool."""
    from src.db.models import Law
    from sqlalchemy import select
    from uuid import UUID

    law_id = arguments.get("id")
    abbreviation = arguments.get("abbreviation")

    if law_id:
        stmt = select(Law).where(Law.id == UUID(law_id))
    elif abbreviation:
        stmt = select(Law).where(Law.abbreviation.ilike(abbreviation))
    else:
        raise ValueError("Either 'id' or 'abbreviation' must be provided")

    result = await session.execute(stmt)
    law = result.scalar_one_or_none()

    if not law:
        return [TextContent(type="text", text="Law not found")]

    response_text = f"**{law.title}**\n\n"
    if law.abbreviation:
        response_text += f"Abbreviation: {law.abbreviation}\n"
    response_text += f"Jurisdiction: {law.jurisdiction.value}\n"
    response_text += f"Type: {law.document_type.value}\n"
    if law.official_number:
        response_text += f"Official Number: {law.official_number}\n"
    if law.publication_date:
        response_text += f"Published: {law.publication_date}\n"
    if law.last_modified_date:
        response_text += f"Last Modified: {law.last_modified_date}\n"
    response_text += f"Source: {law.source_url}\n\n"
    if law.full_text:
        response_text += f"Full Text:\n{law.full_text[:2000]}...\n"

    return [TextContent(type="text", text=response_text)]


async def _search_case_law(session, arguments: Dict[str, Any]) -> List[TextContent]:
    """Execute search_case_law tool."""
    from src.db.models import CaseLaw
    from sqlalchemy import select

    query = arguments["query"]
    jurisdiction = arguments.get("jurisdiction")
    court_level = arguments.get("court_level")
    limit = arguments.get("limit", 10)

    vector_store = VectorStore(session)

    results = await vector_store.search_case_law(
        query=query,
        limit=limit,
        jurisdiction_filter=jurisdiction,
        court_level_filter=court_level,
    )

    response_text = f"Found {len(results)} relevant court decisions for query: '{query}'\n\n"

    for i, result in enumerate(results, 1):
        stmt = select(CaseLaw).where(CaseLaw.id == result["case_law_id"])
        case_result = await session.execute(stmt)
        case_law = case_result.scalar_one_or_none()

        if case_law:
            response_text += f"{i}. {case_law.court} - {case_law.case_number}\n"
            response_text += f"   Decision Date: {case_law.decision_date}\n"
            response_text += f"   Similarity: {result['similarity']:.2%}\n"
            if case_law.title:
                response_text += f"   Title: {case_law.title}\n"
            response_text += f"   Content: {result['content'][:300]}...\n"
            response_text += f"   Source: {case_law.source_url}\n\n"

    return [TextContent(type="text", text=response_text)]


async def _get_legal_changes(session, arguments: Dict[str, Any]) -> List[TextContent]:
    """Execute get_legal_changes tool."""
    from datetime import datetime, timedelta
    from src.db.models import Law
    from sqlalchemy import select, desc

    days = arguments.get("days", 30)
    jurisdiction = arguments.get("jurisdiction")
    limit = arguments.get("limit", 50)

    cutoff_date = datetime.now().date() - timedelta(days=days)

    stmt = select(Law).where(
        (Law.last_modified_date >= cutoff_date) | (Law.publication_date >= cutoff_date)
    )

    if jurisdiction:
        stmt = stmt.where(Law.jurisdiction.in_(jurisdiction))

    stmt = stmt.order_by(desc(Law.last_modified_date)).limit(limit)

    result = await session.execute(stmt)
    laws = result.scalars().all()

    response_text = f"Legal changes in the last {days} days:\n\n"

    for i, law in enumerate(laws, 1):
        change_date = law.last_modified_date or law.publication_date
        response_text += f"{i}. {law.title}"
        if law.abbreviation:
            response_text += f" ({law.abbreviation})"
        response_text += f"\n"
        response_text += f"   Change Date: {change_date}\n"
        response_text += f"   Jurisdiction: {law.jurisdiction.value}\n"
        response_text += f"   Source: {law.source_url}\n\n"

    return [TextContent(type="text", text=response_text)]


async def _get_related_laws(session, arguments: Dict[str, Any]) -> List[TextContent]:
    """Execute get_related_laws tool."""
    from src.db.models import Law
    from sqlalchemy import select
    from uuid import UUID

    law_id = UUID(arguments["law_id"])
    limit = arguments.get("limit", 5)

    # Check if law exists
    stmt = select(Law).where(Law.id == law_id)
    result = await session.execute(stmt)
    law = result.scalar_one_or_none()

    if not law:
        return [TextContent(type="text", text="Law not found")]

    vector_store = VectorStore(session)
    similar_chunks = await vector_store.get_similar_laws(law_id=law_id, limit=limit)

    # Get unique law IDs
    related_law_ids = list({chunk["law_id"] for chunk in similar_chunks})

    response_text = f"Laws related to '{law.title}':\n\n"

    for i, related_law_id in enumerate(related_law_ids[:limit], 1):
        stmt = select(Law).where(Law.id == related_law_id)
        result = await session.execute(stmt)
        related_law = result.scalar_one_or_none()

        if related_law:
            best_similarity = max(
                chunk["similarity"]
                for chunk in similar_chunks
                if chunk["law_id"] == related_law_id
            )

            response_text += f"{i}. {related_law.title}"
            if related_law.abbreviation:
                response_text += f" ({related_law.abbreviation})"
            response_text += f"\n"
            response_text += f"   Similarity: {best_similarity:.2%}\n"
            response_text += f"   Jurisdiction: {related_law.jurisdiction.value}\n"
            response_text += f"   Source: {related_law.source_url}\n\n"

    return [TextContent(type="text", text=response_text)]


@mcp_server.list_resources()
async def list_resources() -> List[Resource]:
    """
    List available MCP resources.

    Returns:
        List of resource definitions
    """
    return [
        Resource(
            uri=AnyUrl("legal://jurisdictions"),
            name="Legal Jurisdictions",
            description="Available legal jurisdictions (federal, eu, bavaria)",
            mimeType="application/json",
        ),
        Resource(
            uri=AnyUrl("legal://document-types"),
            name="Document Types",
            description="Available legal document types",
            mimeType="application/json",
        ),
    ]


@mcp_server.read_resource()
async def read_resource(uri: AnyUrl) -> str:
    """
    Read MCP resource content.

    Args:
        uri: Resource URI

    Returns:
        Resource content as string

    Raises:
        ValueError: If resource not found
    """
    import json

    if str(uri) == "legal://jurisdictions":
        return json.dumps({
            "jurisdictions": ["federal", "eu", "bavaria", "other"],
            "descriptions": {
                "federal": "German Federal Law (Bundesrecht)",
                "eu": "European Union Law",
                "bavaria": "Bavarian State Law (Bayerisches Landesrecht)",
                "other": "Other jurisdictions",
            },
        })
    elif str(uri) == "legal://document-types":
        return json.dumps({
            "document_types": ["law", "regulation", "directive", "decision", "other"],
            "descriptions": {
                "law": "Legislative acts (Gesetze)",
                "regulation": "Regulatory acts (Verordnungen)",
                "directive": "EU Directives",
                "decision": "EU Decisions",
                "other": "Other document types",
            },
        })
    else:
        raise ValueError(f"Unknown resource: {uri}")


async def run_stdio_server():
    """Run MCP server using stdio transport."""
    async with stdio_server() as (read_stream, write_stream):
        await mcp_server.run(
            read_stream,
            write_stream,
            mcp_server.create_initialization_options(),
        )
