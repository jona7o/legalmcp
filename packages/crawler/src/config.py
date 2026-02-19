"""Configuration management for the Legal MCP crawler."""

import os
from typing import List
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Application settings with environment variable support."""
    
    model_config = SettingsConfigDict(
        env_file='.env',
        env_file_encoding='utf-8',
        case_sensitive=False
    )

    # Database settings
    postgres_host: str = Field(default="localhost", alias="POSTGRES_HOST")
    postgres_port: int = Field(default=5432, alias="POSTGRES_PORT")
    postgres_db: str = Field(default="legalmcp", alias="POSTGRES_DB")
    postgres_user: str = Field(default="legal", alias="POSTGRES_USER")
    postgres_password: str = Field(default="legal_dev_password", alias="POSTGRES_PASSWORD")

    # Crawler settings
    crawler_rate_limit: float = Field(default=1.0, alias="CRAWLER_RATE_LIMIT")
    crawler_max_concurrent: int = Field(default=5, alias="CRAWLER_MAX_CONCURRENT")
    crawler_timeout: int = Field(default=30, alias="CRAWLER_TIMEOUT")
    crawler_retry_attempts: int = Field(default=3, alias="CRAWLER_RETRY_ATTEMPTS")
    crawler_user_agent: str = Field(
        default="LegalMCP-Crawler/1.0 (https://github.com/your-org/legalmcp)",
        alias="CRAWLER_USER_AGENT"
    )

    # Embedding model settings
    embedding_model: str = Field(
        default="deepset/gbert-base",
        alias="EMBEDDING_MODEL"
    )
    embedding_batch_size: int = Field(default=8, alias="EMBEDDING_BATCH_SIZE")
    embedding_max_length: int = Field(default=512, alias="EMBEDDING_MAX_LENGTH")
    
    # Chunking settings
    chunk_size: int = Field(default=512, alias="CHUNK_SIZE")
    chunk_overlap: int = Field(default=50, alias="CHUNK_OVERLAP")

    # Logging
    log_level: str = Field(default="INFO", alias="LOG_LEVEL")

    # Cache directory
    cache_dir: str = Field(default="./cache", alias="CACHE_DIR")

    # Data sources enabled
    enable_bundesrecht: bool = Field(default=True, alias="ENABLE_BUNDESRECHT")
    enable_bayern: bool = Field(default=True, alias="ENABLE_BAYERN")
    enable_eurlex: bool = Field(default=True, alias="ENABLE_EURLEX")

    # Source URLs
    bundesrecht_xml_url: str = Field(
        default="https://www.gesetze-im-internet.de/gii-toc.xml",
        alias="BUNDESRECHT_XML_URL"
    )
    bayern_base_url: str = Field(
        default="https://www.gesetze-bayern.de",
        alias="BAYERN_BASE_URL"
    )
    eurlex_sparql_endpoint: str = Field(
        default="https://publications.europa.eu/webapi/rdf/sparql",
        alias="EURLEX_SPARQL_ENDPOINT"
    )
    eurlex_cellar_endpoint: str = Field(
        default="https://eur-lex.europa.eu/legal-content",
        alias="EURLEX_CELLAR_ENDPOINT"
    )

    @property
    def database_url(self) -> str:
        """Construct PostgreSQL database URL."""
        return (
            f"postgresql+asyncpg://{self.postgres_user}:{self.postgres_password}"
            f"@{self.postgres_host}:{self.postgres_port}/{self.postgres_db}"
        )
    
    @property
    def database_url_sync(self) -> str:
        """Construct synchronous PostgreSQL database URL."""
        return (
            f"postgresql://{self.postgres_user}:{self.postgres_password}"
            f"@{self.postgres_host}:{self.postgres_port}/{self.postgres_db}"
        )


# Global settings instance
settings = Settings()
