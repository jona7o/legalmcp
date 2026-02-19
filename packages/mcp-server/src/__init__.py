"""Legal MCP Server - Python MCP Backend."""

__version__ = "0.1.0"
__author__ = "Legal MCP Team"
__description__ = "MCP HTTP Server for German Legal Search"

from src.config import get_settings
from src.main import app

__all__ = ["app", "get_settings"]
