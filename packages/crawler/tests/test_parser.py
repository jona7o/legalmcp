"""Tests for XML and HTML parsers."""

import pytest
from src.processing.parser import XMLParser, HTMLParser


def test_xml_parser_bundesrecht_toc():
    """Test parsing Bundesrecht TOC XML."""
    xml_content = """<?xml version="1.0" encoding="UTF-8"?>
    <toc>
        <item>
            <title>Grundgesetz (GG)</title>
            <link>/gg/xml.zip</link>
        </item>
        <item>
            <title>Bürgerliches Gesetzbuch (BGB)</title>
            <link>/bgb/xml.zip</link>
        </item>
    </toc>
    """
    
    parser = XMLParser()
    items = parser.parse_bundesrecht_toc(xml_content)
    
    assert len(items) == 2
    assert items[0]['title'] == 'Grundgesetz (GG)'
    assert items[0]['abbreviation'] == 'GG'
    assert items[1]['abbreviation'] == 'BGB'


def test_html_parser_extract_text():
    """Test extracting text from HTML."""
    html = """
    <html>
        <body>
            <h1>Test Gesetz</h1>
            <section>
                <h2>§ 1 Allgemeines</h2>
                <p>Dies ist der Gesetzestext.</p>
            </section>
        </body>
    </html>
    """
    
    parser = HTMLParser()
    from bs4 import BeautifulSoup
    soup = BeautifulSoup(html, 'lxml')
    
    text = parser._extract_clean_text(soup)
    
    assert 'Test Gesetz' in text
    assert '§ 1' in text
    assert 'Gesetzestext' in text


def test_html_parser_bayern_law():
    """Test parsing Bayern law HTML."""
    html = """
    <html>
        <body>
            <h1>Bayerisches Beamtengesetz</h1>
            <div class="law-content">
                <section>
                    <h2>Art. 1 Geltungsbereich</h2>
                    <p>Dieses Gesetz gilt für alle Beamten.</p>
                </section>
            </div>
        </body>
    </html>
    """
    
    parser = HTMLParser()
    data = parser.parse_bayern_law(html)
    
    assert data['title'] == 'Bayerisches Beamtengesetz'
    assert len(data['full_text']) > 0
