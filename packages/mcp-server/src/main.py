"""Main FastAPI application for Legal MCP Server."""

import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from src.api.middleware import LoggingMiddleware, RateLimitMiddleware
from src.api.routes import router
from src.config import get_settings
from src.db.connection import close_db, init_db

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

settings = get_settings()


@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Application lifespan manager.

    Handles startup and shutdown events.
    """
    # Startup
    logger.info("Starting Legal MCP Server...")
    logger.info(f"Environment: {settings.environment}")
    logger.info(f"Database: {settings.postgres_host}:{settings.postgres_port}/{settings.postgres_db}")

    # Initialize database connection
    try:
        await init_db()
        logger.info("Database connection initialized")
    except Exception as e:
        logger.error(f"Failed to initialize database: {e}")
        raise

    # Pre-load embedding model
    try:
        from src.search.embeddings import get_embedding_service

        embedding_service = get_embedding_service()
        _ = embedding_service._load_model()
        logger.info(f"Loaded embedding model: {settings.embedding_model}")
    except Exception as e:
        logger.warning(f"Failed to pre-load embedding model: {e}")

    logger.info("Legal MCP Server started successfully")

    yield

    # Shutdown
    logger.info("Shutting down Legal MCP Server...")
    await close_db()
    logger.info("Legal MCP Server stopped")


# Create FastAPI application
app = FastAPI(
    title=settings.app_name,
    version=settings.app_version,
    description="MCP HTTP Server for German Legal Search with semantic similarity",
    lifespan=lifespan,
    docs_url="/docs",
    redoc_url="/redoc",
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins_list,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Add custom middleware
app.add_middleware(LoggingMiddleware)
app.add_middleware(RateLimitMiddleware, requests_per_minute=100)

# Note: API Key middleware is optional for development
# Uncomment to enable API key authentication:
# app.add_middleware(APIKeyMiddleware)

# Include API routes
app.include_router(router, prefix="/api")


@app.get("/")
async def root():
    """Root endpoint with service information."""
    return {
        "service": settings.app_name,
        "version": settings.app_version,
        "status": "healthy",
        "endpoints": {
            "api": "/api",
            "docs": "/docs",
            "redoc": "/redoc",
            "health": "/api/health",
            "mcp": "/mcp",
        },
    }


@app.post("/mcp")
async def mcp_endpoint(request: dict):
    """
    MCP protocol endpoint (HTTP transport).

    This endpoint handles MCP requests over HTTP.
    For stdio transport, use the standalone MCP server.

    Args:
        request: MCP request payload

    Returns:
        MCP response
    """
    # TODO: Implement HTTP MCP transport
    # This is a simplified version - full implementation would handle
    # the complete MCP protocol over HTTP
    return JSONResponse(
        status_code=501,
        content={
            "error": "MCP HTTP transport not yet implemented",
            "detail": "Use stdio transport for now: python -m src.mcp.protocol",
        },
    )


@app.exception_handler(Exception)
async def global_exception_handler(request, exc):
    """
    Global exception handler.

    Args:
        request: FastAPI request
        exc: Exception

    Returns:
        JSON error response
    """
    logger.error(f"Unhandled exception: {exc}", exc_info=True)

    return JSONResponse(
        status_code=500,
        content={
            "error": "Internal server error",
            "detail": str(exc) if settings.environment == "development" else "An error occurred",
        },
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "src.main:app",
        host="0.0.0.0",
        port=settings.mcp_server_port,
        reload=settings.environment == "development",
        log_level=settings.log_level.lower(),
    )
