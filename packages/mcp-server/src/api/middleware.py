"""Middleware for authentication, CORS, and request processing."""

import hashlib
import time
from typing import Callable, Optional

from fastapi import HTTPException, Request, status
from fastapi.responses import JSONResponse
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from starlette.middleware.base import BaseHTTPMiddleware

from src.config import get_settings
from src.db.connection import async_session_maker
from src.db.models import APIKey

settings = get_settings()


class APIKeyMiddleware(BaseHTTPMiddleware):
    """Middleware for API key authentication."""

    async def dispatch(self, request: Request, call_next: Callable):
        """
        Validate API key for protected endpoints.

        Args:
            request: FastAPI request
            call_next: Next middleware/route handler

        Returns:
            Response from next handler or 401 error
        """
        # Skip authentication for health check and docs
        if request.url.path in ["/health", "/docs", "/redoc", "/openapi.json"]:
            return await call_next(request)

        # Get API key from header
        api_key = request.headers.get(settings.api_key_header)

        if not api_key:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"error": "API key required", "detail": f"Missing {settings.api_key_header} header"},
            )

        # Validate API key
        is_valid = await self._validate_api_key(api_key, request.client.host if request.client else None)

        if not is_valid:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"error": "Invalid API key", "detail": "The provided API key is invalid or expired"},
            )

        # Proceed to next handler
        response = await call_next(request)
        return response

    async def _validate_api_key(self, api_key: str, origin: Optional[str] = None) -> bool:
        """
        Validate API key against database.

        Args:
            api_key: Raw API key from request
            origin: Request origin/IP

        Returns:
            True if valid, False otherwise
        """
        # Hash the API key
        key_hash = hashlib.sha256(api_key.encode()).hexdigest()

        async with async_session_maker() as session:
            # Query API key
            stmt = select(APIKey).where(
                APIKey.key_hash == key_hash,
                APIKey.is_active == True,
            )
            result = await session.execute(stmt)
            api_key_obj = result.scalar_one_or_none()

            if not api_key_obj:
                return False

            # Check expiration
            if api_key_obj.expires_at and api_key_obj.expires_at < time.time():
                return False

            # Check allowed origins if specified
            if api_key_obj.allowed_origins and origin:
                if origin not in api_key_obj.allowed_origins:
                    return False

            # Update last used timestamp
            api_key_obj.last_used_at = time.time()
            await session.commit()

            return True


class LoggingMiddleware(BaseHTTPMiddleware):
    """Middleware for request/response logging."""

    async def dispatch(self, request: Request, call_next: Callable):
        """
        Log request and response information.

        Args:
            request: FastAPI request
            call_next: Next middleware/route handler

        Returns:
            Response from next handler
        """
        # Record start time
        start_time = time.time()

        # Process request
        response = await call_next(request)

        # Calculate duration
        duration = (time.time() - start_time) * 1000  # milliseconds

        # Add custom header with processing time
        response.headers["X-Process-Time"] = f"{duration:.2f}ms"

        # Log request (in production, use structured logging)
        if settings.log_level == "DEBUG":
            print(
                f"{request.method} {request.url.path} - "
                f"Status: {response.status_code} - "
                f"Duration: {duration:.2f}ms"
            )

        return response


class RateLimitMiddleware(BaseHTTPMiddleware):
    """Simple in-memory rate limiting middleware."""

    def __init__(self, app, requests_per_minute: int = 60):
        """
        Initialize rate limiter.

        Args:
            app: FastAPI app
            requests_per_minute: Maximum requests per minute per IP
        """
        super().__init__(app)
        self.requests_per_minute = requests_per_minute
        self.request_history: dict = {}

    async def dispatch(self, request: Request, call_next: Callable):
        """
        Check rate limit before processing request.

        Args:
            request: FastAPI request
            call_next: Next middleware/route handler

        Returns:
            Response from next handler or 429 error
        """
        # Skip rate limiting for health check
        if request.url.path == "/health":
            return await call_next(request)

        # Get client IP
        client_ip = request.client.host if request.client else "unknown"
        current_time = time.time()

        # Clean old entries
        self._clean_history(current_time)

        # Check rate limit
        if client_ip in self.request_history:
            timestamps = self.request_history[client_ip]
            recent_requests = [t for t in timestamps if current_time - t < 60]

            if len(recent_requests) >= self.requests_per_minute:
                return JSONResponse(
                    status_code=status.HTTP_429_TOO_MANY_REQUESTS,
                    content={
                        "error": "Rate limit exceeded",
                        "detail": f"Maximum {self.requests_per_minute} requests per minute",
                    },
                )

            self.request_history[client_ip] = recent_requests + [current_time]
        else:
            self.request_history[client_ip] = [current_time]

        return await call_next(request)

    def _clean_history(self, current_time: float) -> None:
        """Clean up old request history entries."""
        for client_ip in list(self.request_history.keys()):
            timestamps = [t for t in self.request_history[client_ip] if current_time - t < 60]
            if timestamps:
                self.request_history[client_ip] = timestamps
            else:
                del self.request_history[client_ip]
