"""Text chunking for embedding generation."""

import re
from typing import List, Dict, Any
import tiktoken


class TextChunker:
    """Chunk legal documents for embedding generation."""
    
    def __init__(
        self,
        max_tokens: int = 512,
        overlap_tokens: int = 50,
        encoding_name: str = "cl100k_base"
    ):
        """
        Initialize chunker.
        
        Args:
            max_tokens: Maximum tokens per chunk
            overlap_tokens: Overlap between chunks for context
            encoding_name: Tokenizer encoding to use
        """
        self.max_tokens = max_tokens
        self.overlap_tokens = overlap_tokens
        self.encoding = tiktoken.get_encoding(encoding_name)
    
    def chunk_document(
        self,
        text: str,
        metadata: Dict[str, Any] = None
    ) -> List[Dict[str, Any]]:
        """
        Chunk document into segments suitable for embedding.
        
        Args:
            text: Full document text
            metadata: Optional metadata to include with chunks
            
        Returns:
            List of chunk dictionaries with 'text', 'token_count', 'metadata'
        """
        if not text or not text.strip():
            return []
        
        # First try to chunk by sections/paragraphs
        sections = self._split_into_sections(text)
        
        chunks = []
        for section in sections:
            section_chunks = self._chunk_section(section, metadata or {})
            chunks.extend(section_chunks)
        
        return chunks
    
    def _split_into_sections(self, text: str) -> List[str]:
        """Split text into logical sections."""
        # Try to split by German legal section markers
        section_patterns = [
            r'\n\s*§\s*\d+[a-z]?\s+',  # § 1, § 2a
            r'\n\s*Art(?:ikel)?\.\s*\d+\s+',  # Art. 1, Artikel 2
            r'\n\s*Artikel\s+\d+\s+',
            r'\n\s*\([0-9]+\)\s+',  # (1), (2)
        ]
        
        # Try each pattern
        for pattern in section_patterns:
            splits = re.split(pattern, text)
            if len(splits) > 1:
                # Rejoin with the delimiter
                sections = []
                matches = re.finditer(pattern, text)
                prev_end = 0
                
                for match in matches:
                    if prev_end > 0:
                        sections.append(text[prev_end:match.start()].strip())
                    prev_end = match.start()
                
                if prev_end < len(text):
                    sections.append(text[prev_end:].strip())
                
                return [s for s in sections if s]
        
        # If no sections found, split by paragraphs
        paragraphs = text.split('\n\n')
        return [p.strip() for p in paragraphs if p.strip()]
    
    def _chunk_section(
        self,
        section: str,
        metadata: Dict[str, Any]
    ) -> List[Dict[str, Any]]:
        """Chunk a single section into smaller pieces if needed."""
        tokens = self.encoding.encode(section)
        
        # If section fits in one chunk, return it
        if len(tokens) <= self.max_tokens:
            return [{
                'text': section,
                'token_count': len(tokens),
                'metadata': metadata.copy()
            }]
        
        # Otherwise, split into overlapping chunks
        chunks = []
        start = 0
        
        while start < len(tokens):
            end = min(start + self.max_tokens, len(tokens))
            chunk_tokens = tokens[start:end]
            chunk_text = self.encoding.decode(chunk_tokens)
            
            chunks.append({
                'text': chunk_text,
                'token_count': len(chunk_tokens),
                'metadata': {
                    **metadata,
                    'chunk_start': start,
                    'chunk_end': end,
                }
            })
            
            # Move start with overlap
            start = end - self.overlap_tokens
            
            # Prevent infinite loop
            if start >= len(tokens) - self.overlap_tokens:
                break
        
        return chunks
    
    def count_tokens(self, text: str) -> int:
        """Count tokens in text."""
        return len(self.encoding.encode(text))
    
    def chunk_with_sections(
        self,
        sections: List[Dict[str, str]],
        base_metadata: Dict[str, Any] = None
    ) -> List[Dict[str, Any]]:
        """
        Chunk pre-parsed sections with their metadata.
        
        Args:
            sections: List of sections with 'title', 'number', 'text'
            base_metadata: Base metadata to include with all chunks
            
        Returns:
            List of chunks with section information
        """
        chunks = []
        base_metadata = base_metadata or {}
        
        for section in sections:
            section_text = section.get('text', '')
            if not section_text:
                continue
            
            section_metadata = {
                **base_metadata,
                'section_title': section.get('title', ''),
                'section_number': section.get('number', ''),
            }
            
            section_chunks = self.chunk_document(section_text, section_metadata)
            chunks.extend(section_chunks)
        
        return chunks
