"""Processing package initialization."""

from .parser import XMLParser, HTMLParser, EURLexParser
from .normalizer import TextNormalizer
from .chunker import TextChunker
from .embedder import Embedder, get_embedder

__all__ = [
    "XMLParser",
    "HTMLParser",
    "EURLexParser",
    "TextNormalizer",
    "TextChunker",
    "Embedder",
    "get_embedder",
]
