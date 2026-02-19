"""Embedding generation using Google Vertex AI."""

import asyncio
import os
from functools import lru_cache
from typing import List

from google.auth import default
from google.auth.credentials import Credentials
from google.oauth2 import service_account
import vertexai
from vertexai.language_models import TextEmbeddingModel

from src.config import get_settings

settings = get_settings()

# Configure Vertex AI
SERVICE_ACCOUNT_FILE = "/app/service-account.json"
GCP_PROJECT_ID = os.getenv("GCP_PROJECT_ID", "innfactory-ai-consulting")
LOCATION = "us-central1"  # Default Vertex AI location

# Initialize Vertex AI with credentials
credentials: Credentials
if os.path.exists(SERVICE_ACCOUNT_FILE):
    credentials = service_account.Credentials.from_service_account_file(
        SERVICE_ACCOUNT_FILE,
        scopes=['https://www.googleapis.com/auth/cloud-platform']
    )
    vertexai.init(project=GCP_PROJECT_ID, location=LOCATION, credentials=credentials)
else:
    # Fallback to default credentials (for local development)
    credentials, project = default()
    vertexai.init(project=GCP_PROJECT_ID, location=LOCATION, credentials=credentials)


class EmbeddingService:
    """Service for generating text embeddings using Google's Vertex AI."""

    def __init__(self):
        """Initialize the embedding service."""
        # Vertex AI is configured at module level
        
        # Use textembedding-gecko for 768-dimensional embeddings
        self.model_name = "text-embedding-004"
        self.model = TextEmbeddingModel.from_pretrained(self.model_name)
        self.dimension = 768

    async def encode_text(self, text: str) -> List[float]:
        """
        Encode a single text into an embedding vector.

        Args:
            text: Text to encode

        Returns:
            List of floats representing the embedding vector (768 dimensions)
        """
        result = await self.encode_texts([text])
        return result[0]

    async def encode_texts(self, texts: List[str]) -> List[List[float]]:
        """
        Encode multiple texts into embedding vectors.

        Args:
            texts: List of texts to encode

        Returns:
            List of embedding vectors
        """
        if not texts:
            return []

        # Run in thread pool since genai might be blocking
        loop = asyncio.get_event_loop()
        embeddings = await loop.run_in_executor(
            None, self._encode_sync, texts
        )
        return embeddings

    def _encode_sync(self, texts: List[str]) -> List[List[float]]:
        """Synchronous encoding using Vertex AI."""
        embeddings = []
        
        for text in texts:
            # Truncate very long texts (Vertex AI has limits)
            truncated_text = text[:20000] if len(text) > 20000 else text
            
            try:
                # Get embedding from Vertex AI
                embedding_result = self.model.get_embeddings([truncated_text])
                embeddings.append(embedding_result[0].values)
                
            except Exception as e:
                print(f"Error encoding text: {e}")
                # Return zero vector as fallback
                embeddings.append([0.0] * self.dimension)
        
        return embeddings

    async def encode_chunks(self, chunks: List[str], batch_size: int = 32) -> List[List[float]]:
        """
        Encode chunks in batches for better performance.

        Args:
            chunks: List of text chunks
            batch_size: Batch size for encoding

        Returns:
            List of embedding vectors
        """
        all_embeddings = []

        for i in range(0, len(chunks), batch_size):
            batch = chunks[i:i + batch_size]
            embeddings = await self.encode_texts(batch)
            all_embeddings.extend(embeddings)

        return all_embeddings


@lru_cache()
def get_embedding_service() -> EmbeddingService:
    """Get cached embedding service instance."""
    return EmbeddingService()
