"""Tests for text chunker."""

import pytest
from src.processing.chunker import TextChunker


def test_chunker_initialization():
    """Test chunker initialization."""
    chunker = TextChunker(max_tokens=100, overlap_tokens=10)
    assert chunker.max_tokens == 100
    assert chunker.overlap_tokens == 10


def test_chunk_short_text():
    """Test chunking text shorter than max tokens."""
    chunker = TextChunker(max_tokens=512)
    text = "Dies ist ein kurzer Test."
    
    chunks = chunker.chunk_document(text)
    
    assert len(chunks) == 1
    assert chunks[0]['text'] == text
    assert chunks[0]['token_count'] > 0


def test_chunk_long_text():
    """Test chunking text longer than max tokens."""
    chunker = TextChunker(max_tokens=50, overlap_tokens=5)
    
    # Create a long text
    text = " ".join(["Dies ist ein langer Satz." for _ in range(100)])
    
    chunks = chunker.chunk_document(text)
    
    assert len(chunks) > 1
    for chunk in chunks:
        assert chunk['token_count'] <= 50


def test_chunk_with_sections():
    """Test chunking with section information."""
    chunker = TextChunker(max_tokens=100)
    
    sections = [
        {
            'title': '§ 1 Allgemeines',
            'number': '§ 1',
            'text': 'Dies ist der Text von Paragraph 1.'
        },
        {
            'title': '§ 2 Besonderes',
            'number': '§ 2',
            'text': 'Dies ist der Text von Paragraph 2.'
        }
    ]
    
    chunks = chunker.chunk_with_sections(sections)
    
    assert len(chunks) >= 2
    assert chunks[0]['metadata']['section_number'] == '§ 1'
    assert chunks[1]['metadata']['section_number'] == '§ 2'


def test_empty_text():
    """Test chunking empty text."""
    chunker = TextChunker()
    chunks = chunker.chunk_document("")
    assert len(chunks) == 0


def test_token_counting():
    """Test token counting."""
    chunker = TextChunker()
    text = "Dies ist ein Test."
    
    count = chunker.count_tokens(text)
    assert count > 0
    assert isinstance(count, int)
