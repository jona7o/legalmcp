"""Configuration management for Legal MCP Server."""

from functools import lru_cache
from typing import List

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Application settings loaded from environment variables."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    # Database Configuration
    postgres_host: str = Field(default="localhost", description="PostgreSQL host")
    postgres_port: int = Field(default=5432, description="PostgreSQL port")
    postgres_db: str = Field(default="legalmcp", description="PostgreSQL database name")
    postgres_user: str = Field(default="legal", description="PostgreSQL user")
    postgres_password: str = Field(default="legal_dev_password", description="PostgreSQL password")

    @property
    def database_url(self) -> str:
        """Construct database URL for SQLAlchemy."""
        return (
            f"postgresql+asyncpg://{self.postgres_user}:{self.postgres_password}"
            f"@{self.postgres_host}:{self.postgres_port}/{self.postgres_db}"
        )

    @property
    def sync_database_url(self) -> str:
        """Construct synchronous database URL for migrations."""
        return (
            f"postgresql+psycopg2://{self.postgres_user}:{self.postgres_password}"
            f"@{self.postgres_host}:{self.postgres_port}/{self.postgres_db}"
        )

    # Server Configuration
    mcp_server_port: int = Field(default=8000, description="Server port")
    log_level: str = Field(default="INFO", description="Logging level")
    environment: str = Field(default="development", description="Environment name")

    # Security Configuration
    api_key_header: str = Field(default="X-API-Key", description="API key header name")
    cors_origins: str = Field(
        default="http://localhost:3000,http://localhost:8000",
        description="Comma-separated CORS origins",
    )

    @property
    def cors_origins_list(self) -> List[str]:
        """Parse CORS origins from comma-separated string."""
        return [origin.strip() for origin in self.cors_origins.split(",")]

    # ML Models Configuration
    embedding_model: str = Field(
        default="deepset/gbert-base", description="Sentence transformer model"
    )
    embedding_dimension: int = Field(default=768, description="Embedding vector dimension")
    model_cache_dir: str = Field(default="/app/models", description="Model cache directory")

    # Search Configuration
    default_search_limit: int = Field(default=10, description="Default search result limit")
    max_search_limit: int = Field(default=100, description="Maximum search result limit")
    min_similarity_score: float = Field(
        default=0.7, description="Minimum similarity score for search results"
    )

    # Application Metadata
    app_name: str = Field(default="Legal MCP Server", description="Application name")
    app_version: str = Field(default="0.1.0", description="Application version")


@lru_cache()
def get_settings() -> Settings:
    """Get cached settings instance."""
    return Settings()
