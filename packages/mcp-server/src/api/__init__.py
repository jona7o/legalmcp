"""API module initialization."""

from src.api.middleware import APIKeyMiddleware, LoggingMiddleware, RateLimitMiddleware
from src.api.routes import router
from src.api.schemas import (
    ErrorResponse,
    HealthResponse,
    SearchLawsRequest,
    SearchLawsResponse,
)

__all__ = [
    "router",
    "APIKeyMiddleware",
    "LoggingMiddleware",
    "RateLimitMiddleware",
    "SearchLawsRequest",
    "SearchLawsResponse",
    "HealthResponse",
    "ErrorResponse",
]
