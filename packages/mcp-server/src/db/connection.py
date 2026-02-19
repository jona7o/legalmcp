"""Database connection management."""

from typing import AsyncGenerator

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncEngine, AsyncSession, create_async_engine
from sqlalchemy.orm import sessionmaker

from src.config import get_settings

settings = get_settings()

# Create async engine
engine: AsyncEngine = create_async_engine(
    settings.database_url,
    echo=settings.log_level == "DEBUG",
    pool_size=20,
    max_overflow=10,
    pool_pre_ping=True,
)

# Create async session factory
async_session_maker = sessionmaker(
    engine,
    class_=AsyncSession,
    expire_on_commit=False,
    autocommit=False,
    autoflush=False,
)


async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """
    Dependency function to get database session.
    
    Yields:
        AsyncSession: Database session
    """
    async with async_session_maker() as session:
        try:
            yield session
        finally:
            await session.close()


async def init_db() -> None:
    """Initialize database connection pool."""
    # Test connection
    async with engine.begin() as conn:
        await conn.execute(text("SELECT 1"))


async def close_db() -> None:
    """Close database connection pool."""
    await engine.dispose()
