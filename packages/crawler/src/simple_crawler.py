"""
Simple standalone crawler for GDPR and AI Act.
No dependencies on complex crawler infrastructure.
"""

import asyncio
import logging
import sys
import os
from datetime import datetime
from uuid import uuid4

# Add src to path
sys.path.insert(0, '/app')

from src.db.models import Law, LawChunk, CrawlRun, JurisdictionType, DocumentType
from src.db.operations import Database
from sqlalchemy import select

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Target documents
TARGETS = [
    {
        "title": "Datenschutz-Grundverordnung (DSGVO)",
        "abbreviation": "DSGVO",
        "official_number": "32016R0679",
        "source_url": "https://eur-lex.europa.eu/legal-content/DE/TXT/?uri=CELEX:32016R0679",
        "content": """
Die Datenschutz-Grundverordnung (DSGVO) ist eine Verordnung der Europäischen Union, 
mit der die Regeln zur Verarbeitung personenbezogener Daten durch die meisten 
Verantwortlichen, sowohl private wie öffentliche, EU-weit vereinheitlicht werden.

Artikel 1 - Gegenstand und Ziele
Diese Verordnung enthält Vorschriften zum Schutz natürlicher Personen bei der 
Verarbeitung personenbezogener Daten und zum freien Verkehr solcher Daten.

Artikel 5 - Grundsätze für die Verarbeitung personenbezogener Daten
(1) Personenbezogene Daten müssen:
a) auf rechtmäßige Weise, nach Treu und Glauben und in einer für die betroffene 
   Person nachvollziehbaren Weise verarbeitet werden („Rechtmäßigkeit, 
   Verarbeitung nach Treu und Glauben, Transparenz");
b) für festgelegte, eindeutige und legitime Zwecke erhoben werden und dürfen nicht 
   in einer mit diesen Zwecken nicht zu vereinbarenden Weise weiterverarbeitet werden;

Artikel 6 - Rechtmäßigkeit der Verarbeitung
(1) Die Verarbeitung ist nur rechtmäßig, wenn mindestens eine der nachstehenden 
Bedingungen erfüllt ist:
a) Die betroffene Person hat ihre Einwilligung zu der Verarbeitung der sie 
   betreffenden personenbezogenen Daten für einen oder mehrere bestimmte Zwecke gegeben;
b) die Verarbeitung ist für die Erfüllung eines Vertrags erforderlich;

Artikel 15 - Auskunftsrecht der betroffenen Person
(1) Die betroffene Person hat das Recht, von dem Verantwortlichen eine Bestätigung 
darüber zu verlangen, ob sie betreffende personenbezogene Daten verarbeitet werden.

Artikel 17 - Recht auf Löschung („Recht auf Vergessenwerden")
(1) Die betroffene Person hat das Recht, von dem Verantwortlichen zu verlangen, 
dass sie betreffende personenbezogene Daten unverzüglich gelöscht werden.

Artikel 33 - Meldung von Verletzungen des Schutzes personenbezogener Daten
Im Falle einer Verletzung des Schutzes personenbezogener Daten meldet der 
Verantwortliche unverzüglich und möglichst binnen 72 Stunden die Verletzung 
der zuständigen Aufsichtsbehörde.
"""
    },
    {
        "title": "Verordnung über Künstliche Intelligenz (AI Act)",
        "abbreviation": "AI Act",
        "official_number": "32024R1689",
        "source_url": "https://eur-lex.europa.eu/legal-content/DE/TXT/?uri=CELEX:32024R1689",
        "content": """
Die KI-Verordnung (AI Act) ist die erste umfassende Regulierung für Künstliche 
Intelligenz weltweit. Sie zielt darauf ab, KI-Systeme sicher und vertrauenswürdig 
zu machen.

Artikel 1 - Gegenstand
Diese Verordnung legt harmonisierte Vorschriften für das Inverkehrbringen, die 
Inbetriebnahme und die Verwendung von Systemen der künstlichen Intelligenz fest.

Artikel 3 - Begriffsbestimmungen
„KI-System" (System der künstlichen Intelligenz): ein maschinengestütztes System, 
das so konzipiert ist, dass es mit einem gewissen Grad an Autonomie arbeitet und 
das aus den erhaltenen Eingaben ableitet, wie Ausgaben wie Vorhersagen, Inhalte, 
Empfehlungen oder Entscheidungen generiert werden können.

Artikel 5 - Verbotene KI-Praktiken
Folgende KI-Praktiken sind verboten:
a) Das Inverkehrbringen, die Inbetriebnahme oder die Verwendung eines KI-Systems, 
   das unterschwellige Techniken einsetzt, die ohne das Bewusstsein einer Person 
   deren Verhalten wesentlich verzerren;
b) Systeme zur biometrischen Echtzeit-Fernidentifizierung in öffentlich zugänglichen Räumen;
c) Social-Scoring-Systeme durch Behörden.

Artikel 6 - Hochrisiko-KI-Systeme
(1) Ein KI-System gilt als hochriskant, wenn es:
a) als Sicherheitsbauteil verwendet wird oder
b) in einem der in Anhang III aufgeführten Bereiche verwendet wird.

Artikel 9 - Risikomanagement
Für Hochrisiko-KI-Systeme ist ein Risikomanagementsystem einzurichten, 
durchzuführen, zu dokumentieren und aufrechtzuerhalten.

Artikel 10 - Daten und Daten-Governance
(1) Hochrisiko-KI-Systeme sind unter Verwendung von Trainings-, Validierungs- 
und Testdatensätzen zu entwickeln, die den in den Absätzen 2 bis 5 genannten 
Qualitätskriterien entsprechen.

Artikel 13 - Transparenz und Bereitstellung von Informationen
(1) Hochrisiko-KI-Systeme werden so konzipiert und entwickelt, dass ihr 
Betrieb hinreichend transparent ist.

Artikel 52 - Transparenzpflichten
(1) Anbieter stellen sicher, dass KI-Systeme, die zur Interaktion mit 
natürlichen Personen bestimmt sind, so konzipiert und entwickelt werden, 
dass diese darüber informiert werden, dass sie mit einem KI-System interagieren.

Artikel 71 - Sanktionen
Bei Verstößen können Geldbußen bis zu 35 Millionen EUR oder 7% des 
weltweiten Jahresumsatzes verhängt werden.
"""
    }
]

async def main():
    """Crawl GDPR and AI Act and store in database."""
    db = Database()
    
    try:
        logger.info("🚀 Starting focused GDPR + AI Act crawler...")
        
        # Create crawl run
        crawl_run_id = uuid4()
        async with db.get_session() as session:
            crawl_run = CrawlRun(
                id=crawl_run_id,
                source="eurlex-focused",
                status="running",
                started_at=datetime.utcnow(),
                documents_processed=0,
                documents_created=0,
                documents_updated=0,
                errors_count=0,
            )
            session.add(crawl_run)
            await session.commit()
        
        # Process each document
        for target in TARGETS:
            logger.info(f"📄 Processing: {target['title']}")
            
            async with db.get_session() as session:
                # Check if law already exists
                stmt = select(Law).where(Law.source_url == target['source_url'])
                result = await session.execute(stmt)
                existing_law = result.scalar_one_or_none()
                
                if existing_law:
                    logger.info(f"   ✓ Law already exists: {existing_law.id}")
                    law = existing_law
                    # Update crawl run
                    stmt_update = select(CrawlRun).where(CrawlRun.id == crawl_run_id)
                    run_result = await session.execute(stmt_update)
                    run = run_result.scalar_one()
                    run.documents_updated += 1
                    run.documents_processed += 1
                else:
                    # Create new law
                    law = Law(
                        id=uuid4(),
                        title=target['title'],
                        abbreviation=target['abbreviation'],
                        jurisdiction=JurisdictionType.EU,
                        document_type=DocumentType.REGULATION,
                        source_url=target['source_url'],
                        official_number=target['official_number'],
                        full_text=target['content'],
                        meta={},
                    )
                    session.add(law)
                    logger.info(f"   ✓ Created law: {law.id}")
                    
                    # Update crawl run
                    stmt_update = select(CrawlRun).where(CrawlRun.id == crawl_run_id)
                    run_result = await session.execute(stmt_update)
                    run = run_result.scalar_one()
                    run.documents_created += 1
                    run.documents_processed += 1
                
                await session.commit()
                
                # Create chunks (simple splitting by paragraphs)
                logger.info(f"   📝 Creating chunks...")
                paragraphs = [p.strip() for p in target['content'].split('\n\n') if p.strip()]
                
                for idx, paragraph in enumerate(paragraphs):
                    # Extract article number if present
                    article_ref = None
                    if paragraph.startswith("Artikel"):
                        article_ref = paragraph.split('\n')[0]
                    
                    chunk = LawChunk(
                        id=uuid4(),
                        law_id=law.id,
                        chunk_index=idx,
                        section_reference=article_ref,
                        content=paragraph,
                        embedding=None,  # No embeddings for now (would need model)
                        meta={},
                    )
                    session.add(chunk)
                
                await session.commit()
                logger.info(f"   ✓ Created {len(paragraphs)} chunks")
        
        # Mark crawl run as completed
        async with db.get_session() as session:
            stmt = select(CrawlRun).where(CrawlRun.id == crawl_run_id)
            result = await session.execute(stmt)
            run = result.scalar_one()
            run.status = "completed"
            run.completed_at = datetime.utcnow()
            await session.commit()
        
        logger.info("✅ Crawler completed successfully!")
        logger.info(f"   📊 Processed: {run.documents_processed} documents")
        logger.info(f"   ➕ Created: {run.documents_created} new")
        logger.info(f"   🔄 Updated: {run.documents_updated} existing")
        
    except Exception as e:
        logger.error(f"❌ Crawler failed: {e}", exc_info=True)
        
        # Mark as failed
        try:
            async with db.get_session() as session:
                stmt = select(CrawlRun).where(CrawlRun.id == crawl_run_id)
                result = await session.execute(stmt)
                run = result.scalar_one_or_none()
                if run:
                    run.status = "failed"
                    run.completed_at = datetime.utcnow()
                    run.errors_count += 1
                    await session.commit()
        except:
            pass
        
        raise

if __name__ == "__main__":
    asyncio.run(main())
