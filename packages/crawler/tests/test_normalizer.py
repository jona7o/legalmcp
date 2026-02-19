"""Tests for text normalizer."""

import pytest
from src.processing.normalizer import TextNormalizer


def test_normalize_encoding():
    """Test encoding normalization."""
    text = "MÃ¼nchen Ã¤Ã¶Ã¼ÃŸ"
    normalized = TextNormalizer.normalize(text)
    assert "ü" in normalized
    assert "ä" in normalized
    assert "ö" in normalized


def test_normalize_whitespace():
    """Test whitespace normalization."""
    text = "Dies  ist   ein    Test.\n\n\n\nNeue Zeile."
    normalized = TextNormalizer.normalize(text)
    
    assert "  " not in normalized
    assert "\n\n\n" not in normalized


def test_normalize_punctuation():
    """Test punctuation normalization."""
    text = "Test–mit—verschiedenen"dashes"und'quotes'"
    normalized = TextNormalizer.normalize(text)
    
    assert "–" not in normalized
    assert "—" not in normalized


def test_extract_section_reference():
    """Test section reference extraction."""
    text = "Gemäß § 1 BGB ist dies so geregelt."
    ref = TextNormalizer.extract_section_reference(text)
    assert ref is not None
    assert "§" in ref
    assert "1" in ref


def test_normalize_empty():
    """Test normalizing empty text."""
    normalized = TextNormalizer.normalize("")
    assert normalized == ""


def test_normalize_none():
    """Test normalizing None."""
    normalized = TextNormalizer.normalize(None)
    assert normalized == ""
