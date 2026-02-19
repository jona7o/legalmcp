"""Shared test fixtures."""

import pytest
import asyncio


@pytest.fixture(scope="session")
def event_loop():
    """Create event loop for async tests."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture
def sample_legal_text():
    """Sample legal text for testing."""
    return """
    § 1 Allgemeine Bestimmungen
    
    (1) Dieses Gesetz regelt die grundlegenden Bestimmungen.
    
    (2) Es gilt für alle Bürger der Bundesrepublik Deutschland.
    
    § 2 Besondere Regelungen
    
    (1) Besondere Regelungen ergeben sich aus den nachfolgenden Absätzen.
    
    (2) Im Einzelfall können Ausnahmen gewährt werden.
    """


@pytest.fixture
def sample_bundesrecht_xml():
    """Sample Bundesrecht XML for testing."""
    return """<?xml version="1.0" encoding="UTF-8"?>
    <toc>
        <item>
            <title>Grundgesetz für die Bundesrepublik Deutschland (GG)</title>
            <link>/gg/xml.zip</link>
        </item>
    </toc>
    """
