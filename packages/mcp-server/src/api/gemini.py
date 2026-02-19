"""Gemini Chat integration for Legal MCP using Vertex AI."""

import os
from typing import List, Dict, Any
from pydantic import BaseModel

from google.auth import default
from google.auth.credentials import Credentials
from google.oauth2 import service_account
import vertexai
from vertexai.generative_models import GenerativeModel, Part, Content

# Configure Vertex AI with Service Account
SERVICE_ACCOUNT_FILE = "/app/service-account.json"
GCP_PROJECT_ID = os.getenv("GCP_PROJECT_ID", "innfactory-ai-consulting")
GEMINI_MODEL = os.getenv("GEMINI_MODEL", "gemini-2.5-pro")
LOCATION = "us-central1"

# Initialize Vertex AI with credentials
credentials: Credentials
if os.path.exists(SERVICE_ACCOUNT_FILE):
    credentials = service_account.Credentials.from_service_account_file(
        SERVICE_ACCOUNT_FILE,
        scopes=['https://www.googleapis.com/auth/cloud-platform']
    )
    vertexai.init(project=GCP_PROJECT_ID, location=LOCATION, credentials=credentials)
else:
    # Fallback to default credentials
    credentials, project = default()
    vertexai.init(project=GCP_PROJECT_ID, location=LOCATION, credentials=credentials)


class ChatMessage(BaseModel):
    """Chat message model."""
    role: str  # 'user' or 'model'
    content: str


class ChatRequest(BaseModel):
    """Chat request with context."""
    message: str
    context: List[Dict[str, Any]] = []  # Search results as context
    history: List[ChatMessage] = []


class ChatResponse(BaseModel):
    """Chat response with sources."""
    answer: str
    sources: List[Dict[str, str]] = []
    model_used: str = GEMINI_MODEL


class GeminiService:
    """Service for Gemini chat integration."""
    
    def __init__(self):
        # Validation is done at module level
        
        # System instruction for legal assistant
        self.system_instruction = """Du bist ein juristischer Assistent für deutsches und europäisches Recht.

🚨 ABSOLUT VERBOTEN 🚨:
- Du darfst NIEMALS eigenes Rechtswissen oder Informationen außerhalb des bereitgestellten Context verwenden
- Du darfst NIEMALS rechtliche Aussagen treffen, die nicht direkt aus den bereitgestellten Rechtstexten stammen
- Du darfst KEINE allgemeinen rechtlichen Erklärungen geben, ohne sie mit konkreten Textstellen zu belegen

✅ PFLICHT-REGELN:
1. Antworte AUSSCHLIESSLICH basierend auf den bereitgestellten Rechtstexten im Context
2. Jede einzelne Aussage MUSS mit einer spezifischen Quelle belegt werden
3. Verwende für JEDE Rechtsaussage die exakte Fundstelle: [Rechtsakt - Artikel/Paragraph Nr.]
4. Wenn die bereitgestellten Rechtstexte die Frage nicht beantworten können, sage klar:
   "Die bereitgestellten Rechtstexte enthalten keine Information zu dieser Frage."
5. Bei unvollständigen Informationen im Context, weise explizit darauf hin
6. Zitiere wörtlich oder paraphrasiere nah am Originaltext
7. Kennzeichne jede Quelle mit: Rechtsakt, Artikel/Paragraph, und Rechtsgebiet (EU/Federal/Bavaria)

📋 ANTWORT-FORMAT:
1. Direkte Antwort auf die Frage mit sofortiger Quellenangabe
2. Wörtliches oder sinngemäßes Zitat der relevanten Rechtsnorm
3. Fundstelle im Format: [DSGVO - Art. 5 Abs. 1 lit. a] oder [AI Act - Art. 6 Abs. 2]
4. Kurze Erklärung in einfacher Sprache (NUR wenn aus dem Gesetzestext ableitbar)
5. Am Ende: Liste aller verwendeten Quellen mit vollständiger Fundstelle und URL

🔍 ZITATFORMAT:
- Verwende für jeden Verweis das Format: [Abkürzung - Art./§ Nr.]
- Beispiel: "Nach [DSGVO - Art. 5 Abs. 1 lit. a] müssen personenbezogene Daten..."
- Beispiel: "Der [AI Act - Art. 5 lit. b] verbietet Social Scoring..."

⚖️ HAFTUNGSAUSSCHLUSS:
Diese Information basiert ausschließlich auf den bereitgestellten Rechtstexten und ersetzt KEINE anwaltliche Beratung. 
Bei rechtlichen Problemen wende dich bitte an einen qualifizierten Rechtsanwalt.
"""
        
        self.model = GenerativeModel(
            GEMINI_MODEL,
            system_instruction=[self.system_instruction]
        )
    
    def _format_context(self, context: List[Dict[str, Any]]) -> str:
        """Format search results as context for Gemini."""
        if not context:
            return "⚠️ Keine Rechtstexte im Context verfügbar. Bitte informiere den Nutzer, dass du keine spezifischen Rechtstexte zur Verfügung hast."
        
        formatted = "=== VERFÜGBARE RECHTSTEXTE (Context) ===\n\n"
        
        for i, item in enumerate(context, 1):
            title = item.get('title', 'Unbekannt')
            formatted += f"📄 [{i}] {title}\n"
            
            metadata = item.get('metadata', {})
            if metadata.get('abbreviation'):
                formatted += f"   Abkürzung: {metadata['abbreviation']}\n"
            
            section_ref = metadata.get('section_reference')
            if section_ref:
                formatted += f"   📍 Fundstelle: {section_ref}\n"
            
            jurisdiction = item.get('jurisdiction', 'unknown').upper()
            formatted += f"   🌍 Rechtsgebiet: {jurisdiction}\n"
            formatted += f"   📋 Typ: {metadata.get('document_type', 'unknown')}\n"
            
            # Content
            content = item.get('content', '')
            if len(content) > 1500:
                content = content[:1500] + "... [gekürzt]"
            formatted += f"\n   Inhalt:\n   {content}\n"
            
            formatted += f"   🔗 Quelle: {metadata.get('source_url', 'N/A')}\n"
            formatted += "\n" + "─"*80 + "\n\n"
        
        return formatted
    
    def _extract_sources(self, context: List[Dict[str, Any]]) -> List[Dict[str, str]]:
        """Extract sources from context."""
        sources = []
        for item in context:
            metadata = item.get('metadata', {})
            source = {
                'title': item.get('title', 'Unbekannt'),
                'jurisdiction': item.get('jurisdiction', 'unknown').upper(),
            }
            
            if metadata.get('abbreviation'):
                source['abbreviation'] = metadata['abbreviation']
            if metadata.get('section_reference'):
                source['section'] = metadata['section_reference']
            if metadata.get('source_url'):
                source['url'] = metadata['source_url']
            
            sources.append(source)
        
        return sources
    
    async def chat(self, request: ChatRequest) -> ChatResponse:
        """
        Chat with Gemini using RAG pattern.
        
        Args:
            request: Chat request with message, context, and history
            
        Returns:
            Chat response with answer and sources
        """
        try:
            # Format context
            context_text = self._format_context(request.context)
            
            # Build conversation history for Gemini (Vertex AI format)
            history_contents = []
            for msg in request.history[-10:]:  # Last 10 messages
                role = "user" if msg.role == "user" else "model"
                history_contents.append(
                    Content(role=role, parts=[Part.from_text(msg.content)])
                )
            
            # Build prompt with context
            prompt = f"""{context_text}

=== NUTZER-FRAGE ===
{request.message}

🚨 ERINNERUNG: Beantworte die Frage AUSSCHLIESSLICH basierend auf den obigen Rechtstexten.
- Verwende KEINE eigenen Informationen oder allgemeines Rechtswissen
- Zitiere für JEDE Aussage die exakte Fundstelle im Format: [Abkürzung - Art./§ Nr.]
- Falls die Frage mit den bereitgestellten Texten nicht beantwortet werden kann, sage das klar und deutlich
- Zitiere wörtlich oder sehr nah am Originaltext"""
            
            # Create chat session with history
            chat = self.model.start_chat(history=history_contents)
            
            # Get response
            response = chat.send_message(prompt)
            
            # Extract sources
            sources = self._extract_sources(request.context)
            
            return ChatResponse(
                answer=response.text,
                sources=sources,
                model_used=GEMINI_MODEL
            )
            
        except Exception as e:
            # Fallback response with detailed error
            import traceback
            error_details = traceback.format_exc()
            return ChatResponse(
                answer=f"⚠️ Fehler bei der Kommunikation mit Gemini: {str(e)}\n\nBitte versuche es erneut oder kontaktiere den Support.",
                sources=[],
                model_used=GEMINI_MODEL
            )


# Global service instance
_gemini_service = None


def get_gemini_service() -> GeminiService:
    """Get or create Gemini service singleton."""
    global _gemini_service
    if _gemini_service is None:
        _gemini_service = GeminiService()
    return _gemini_service
