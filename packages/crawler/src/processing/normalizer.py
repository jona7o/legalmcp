"""Text normalizer for legal documents."""

import re
from typing import Optional


class TextNormalizer:
    """Normalize and clean legal text."""
    
    @staticmethod
    def normalize(text: str) -> str:
        """
        Normalize legal text for consistent processing.
        
        Args:
            text: Raw text to normalize
            
        Returns:
            Normalized text
        """
        if not text:
            return ""
        
        # Fix common German encoding issues
        text = TextNormalizer._fix_encoding(text)
        
        # Normalize punctuation
        text = TextNormalizer._normalize_punctuation(text)
        
        # Fix whitespace
        text = TextNormalizer._normalize_whitespace(text)
        
        return text.strip()
    
    @staticmethod
    def _fix_encoding(text: str) -> str:
        """Fix common encoding issues in German text."""
        replacements = {
            'Ã¤': 'ä', 'Ã¶': 'ö', 'Ã¼': 'ü',
            'Ã„': 'Ä', 'Ã–': 'Ö', 'Ãœ': 'Ü',
            'ÃŸ': 'ß',
            '\xa0': ' ',  # non-breaking space
        }
        
        for old, new in replacements.items():
            text = text.replace(old, new)
        
        return text
    
    @staticmethod
    def _normalize_punctuation(text: str) -> str:
        """Normalize punctuation marks."""
        # Normalize dashes
        text = text.replace('–', '-').replace('—', '-')
        
        # Normalize quotes
        text = text.replace('"', '"').replace('"', '"')
        text = text.replace(''', "'").replace(''', "'")
        text = text.replace('„', '"').replace('"', '"')
        
        return text
    
    @staticmethod
    def _normalize_whitespace(text: str) -> str:
        """Normalize whitespace."""
        # Replace multiple spaces with single space
        text = re.sub(r' +', ' ', text)
        
        # Normalize line breaks (max 2 consecutive)
        text = re.sub(r'\n\s*\n\s*\n+', '\n\n', text)
        
        # Remove trailing whitespace from lines
        text = '\n'.join(line.rstrip() for line in text.split('\n'))
        
        return text
    
    @staticmethod
    def extract_section_reference(text: str) -> Optional[str]:
        """
        Extract legal section reference from text.
        
        Examples: § 1, Art. 5, Abs. 2
        """
        patterns = [
            r'§\s*\d+[a-z]?(?:\s+Abs\.\s*\d+)?',
            r'Art(?:ikel)?\.\s*\d+[a-z]?',
            r'Abs(?:atz)?\.\s*\d+',
        ]
        
        for pattern in patterns:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                return match.group(0)
        
        return None
