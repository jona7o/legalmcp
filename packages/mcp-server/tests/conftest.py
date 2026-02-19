"""Test configuration and fixtures."""

import pytest
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine
from sqlalchemy.orm import sessionmaker

from src.config import get_settings
from src.db.models import Base


@pytest.fixture(scope="session")
def settings():
    """Get test settings."""
    return get_settings()


@pytest.fixture(scope="session")
async def engine(settings):
    """Create test database engine."""
    test_engine = create_async_engine(
        settings.database_url,
        echo=True,
    )

    # Create tables
    async with test_engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    yield test_engine

    # Drop tables
    async with test_engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)

    await test_engine.dispose()


@pytest.fixture
async def db_session(engine):
    """Create test database session."""
    async_session_maker = sessionmaker(
        engine,
        class_=AsyncSession,
        expire_on_commit=False,
    )

    async with async_session_maker() as session:
        yield session
        await session.rollback()
