"""Text parsing utilities for legal documents."""

import re
from typing import Dict, List, Optional, Tuple
from bs4 import BeautifulSoup, Tag
import xml.etree.ElementTree as ET


class XMLParser:
    """Parser for XML legal documents."""
    
    @staticmethod
    def parse_bundesrecht_toc(xml_content: str) -> List[Dict[str, str]]:
        """
        Parse the Bundesrecht table of contents XML.
        
        Returns:
            List of dictionaries with 'title', 'abbreviation', 'link' keys.
        """
        root = ET.fromstring(xml_content)
        items = []
        
        for item in root.findall('.//item'):
            title = item.find('title')
            link = item.find('link')
            
            if title is not None and link is not None:
                items.append({
                    'title': title.text or '',
                    'link': link.text or '',
                    'abbreviation': XMLParser._extract_abbreviation(title.text or '')
                })
        
        return items
    
    @staticmethod
    def _extract_abbreviation(title: str) -> str:
        """Extract abbreviation from title (usually in parentheses)."""
        match = re.search(r'\(([A-Za-zÄÖÜäöüß0-9]+)\)', title)
        return match.group(1) if match else ''
    
    @staticmethod
    def parse_bundesrecht_xml(xml_content: str) -> Dict[str, any]:
        """
        Parse individual Bundesrecht law XML.
        
        Returns:
            Dictionary with parsed document data.
        """
        root = ET.fromstring(xml_content)
        
        # Extract metadata
        metadata = {
            'sections': [],
            'title': '',
            'full_text': '',
        }
        
        # Try to find title/name
        title_elem = root.find('.//TITEL') or root.find('.//titel')
        if title_elem is not None:
            metadata['title'] = title_elem.text or ''
        
        # Extract all text content
        text_parts = []
        for elem in root.iter():
            if elem.text and elem.text.strip():
                text_parts.append(elem.text.strip())
        
        metadata['full_text'] = '\n\n'.join(text_parts)
        
        return metadata


class HTMLParser:
    """Parser for HTML legal documents."""
    
    @staticmethod
    def parse_bayern_law(html_content: str) -> Dict[str, any]:
        """
        Parse Bayern.Recht law page.
        
        Returns:
            Dictionary with parsed document data.
        """
        soup = BeautifulSoup(html_content, 'lxml')
        
        data = {
            'title': '',
            'sections': [],
            'full_text': '',
            'metadata': {}
        }
        
        # Extract title
        title_elem = soup.find('h1') or soup.find('title')
        if title_elem:
            data['title'] = title_elem.get_text(strip=True)
        
        # Extract main content
        content_div = (
            soup.find('div', class_='law-content') or
            soup.find('div', class_='content') or
            soup.find('main') or
            soup.find('article')
        )
        
        if content_div:
            # Extract sections
            sections = content_div.find_all(['section', 'div'], class_=re.compile(r'(section|paragraph|article)'))
            
            for section in sections:
                section_data = HTMLParser._parse_section(section)
                if section_data:
                    data['sections'].append(section_data)
            
            # Get full text
            data['full_text'] = HTMLParser._extract_clean_text(content_div)
        
        return data
    
    @staticmethod
    def _parse_section(section_elem: Tag) -> Optional[Dict[str, str]]:
        """Parse a single section element."""
        section_num = section_elem.find(['h2', 'h3', 'h4', 'span'], class_=re.compile(r'(number|num)'))
        section_title = section_elem.find(['h2', 'h3', 'h4'], class_=re.compile(r'(title|heading)'))
        
        return {
            'number': section_num.get_text(strip=True) if section_num else '',
            'title': section_title.get_text(strip=True) if section_title else '',
            'text': HTMLParser._extract_clean_text(section_elem)
        }
    
    @staticmethod
    def _extract_clean_text(element: Tag) -> str:
        """Extract clean text from HTML element."""
        # Remove script and style elements
        for script in element(['script', 'style', 'nav', 'header', 'footer']):
            script.decompose()
        
        # Get text and clean it
        text = element.get_text(separator='\n', strip=True)
        
        # Clean up whitespace
        text = re.sub(r'\n\s*\n', '\n\n', text)
        text = re.sub(r' +', ' ', text)
        
        return text.strip()


class EURLexParser:
    """Parser for EUR-Lex documents."""
    
    @staticmethod
    def parse_sparql_results(json_data: Dict) -> List[Dict[str, str]]:
        """
        Parse SPARQL query results from EUR-Lex.
        
        Returns:
            List of documents with metadata.
        """
        results = []
        
        if 'results' in json_data and 'bindings' in json_data['results']:
            for binding in json_data['results']['bindings']:
                result = {}
                for key, value in binding.items():
                    if 'value' in value:
                        result[key] = value['value']
                results.append(result)
        
        return results
    
    @staticmethod
    def parse_eurlex_html(html_content: str) -> Dict[str, any]:
        """
        Parse EUR-Lex legal document HTML.
        
        Returns:
            Dictionary with parsed document data.
        """
        soup = BeautifulSoup(html_content, 'lxml')
        
        data = {
            'title': '',
            'celex': '',
            'sections': [],
            'full_text': '',
            'metadata': {}
        }
        
        # Extract title
        title_elem = soup.find('h1', class_='title') or soup.find('h1')
        if title_elem:
            data['title'] = title_elem.get_text(strip=True)
        
        # Extract CELEX number
        celex_elem = soup.find(string=re.compile(r'[0-9]{4}[A-Z][0-9]{4}'))
        if celex_elem:
            data['celex'] = celex_elem.strip()
        
        # Extract main content
        content_div = (
            soup.find('div', id='text') or
            soup.find('div', class_='eli-main-content') or
            soup.find('main')
        )
        
        if content_div:
            # Extract articles
            articles = content_div.find_all(['div', 'section'], class_=re.compile(r'(article|art)'))
            
            for article in articles:
                article_data = EURLexParser._parse_article(article)
                if article_data:
                    data['sections'].append(article_data)
            
            # Get full text
            data['full_text'] = HTMLParser._extract_clean_text(content_div)
        
        return data
    
    @staticmethod
    def _parse_article(article_elem: Tag) -> Optional[Dict[str, str]]:
        """Parse a single article element."""
        article_num = article_elem.find(['span', 'div'], class_=re.compile(r'(num|number)'))
        article_title = article_elem.find(['p', 'div'], class_=re.compile(r'(title|heading)'))
        
        return {
            'number': article_num.get_text(strip=True) if article_num else '',
            'title': article_title.get_text(strip=True) if article_title else '',
            'text': HTMLParser._extract_clean_text(article_elem)
        }


class TextNormalizer:
    """Normalize legal text for processing."""
    
    @staticmethod
    def normalize(text: str) -> str:
        """
        Normalize legal text.
        
        - Fix common encoding issues
        - Normalize whitespace
        - Remove special characters (except legal ones)
        """
        # Fix common German encoding issues
        text = text.replace('Ã¤', 'ä').replace('Ã¶', 'ö').replace('Ã¼', 'ü')
        text = text.replace('Ã„', 'Ä').replace('Ã–', 'Ö').replace('Ãœ', 'Ü')
        text = text.replace('ÃŸ', 'ß')
        
        # Normalize different types of dashes
        text = text.replace('–', '-').replace('—', '-')
        
        # Normalize quotes
        text = text.replace('"', '"').replace('"', '"')
        text = text.replace(''', "'").replace(''', "'")
        
        # Fix whitespace
        text = re.sub(r'\s+', ' ', text)
        text = re.sub(r'\n\s*\n', '\n\n', text)
        
        # Remove non-breaking spaces
        text = text.replace('\xa0', ' ')
        
        return text.strip()
    
    @staticmethod
    def extract_section_number(text: str) -> Optional[str]:
        """Extract section/paragraph number from text."""
        # German patterns: § 1, Art. 5, Abs. 2
        patterns = [
            r'§\s*(\d+[a-z]?)',
            r'Art\.\s*(\d+[a-z]?)',
            r'Abs\.\s*(\d+)',
            r'Artikel\s+(\d+)',
        ]
        
        for pattern in patterns:
            match = re.search(pattern, text)
            if match:
                return match.group(0)
        
        return None
