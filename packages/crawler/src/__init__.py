"""Legal MCP Crawler package."""

__version__ = "0.1.0"

from .config import settings
from .db import db
from .sources import (
    BundesrechtCrawler,
    BayernCrawler,
    EURLexCrawler,
)

__all__ = [
    "settings",
    "db",
    "BundesrechtCrawler",
    "BayernCrawler",
    "EURLexCrawler",
]
