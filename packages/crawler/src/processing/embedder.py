"""Embedding generation for legal text."""

import logging
from typing import List, Optional
import numpy as np
from sentence_transformers import SentenceTransformer
import torch

from ..config import settings

logger = logging.getLogger(__name__)


class Embedder:
    """Generate embeddings for legal text using German Legal BERT."""
    
    def __init__(
        self,
        model_name: Optional[str] = None,
        batch_size: Optional[int] = None,
        device: Optional[str] = None
    ):
        """
        Initialize embedder.
        
        Args:
            model_name: Hugging Face model name
            batch_size: Batch size for encoding
            device: Device to use ('cuda' or 'cpu')
        """
        self.model_name = model_name or settings.embedding_model
        self.batch_size = batch_size or settings.embedding_batch_size
        
        # Auto-detect device
        if device is None:
            self.device = "cuda" if torch.cuda.is_available() else "cpu"
        else:
            self.device = device
        
        logger.info(f"Loading embedding model: {self.model_name} on {self.device}")
        
        self.model = SentenceTransformer(self.model_name, device=self.device)
        self.embedding_dim = self.model.get_sentence_embedding_dimension()
        
        logger.info(f"Model loaded. Embedding dimension: {self.embedding_dim}")
    
    def embed_texts(self, texts: List[str]) -> List[List[float]]:
        """
        Generate embeddings for a list of texts.
        
        Args:
            texts: List of text strings
            
        Returns:
            List of embedding vectors
        """
        if not texts:
            return []
        
        try:
            # Encode texts in batches
            embeddings = self.model.encode(
                texts,
                batch_size=self.batch_size,
                show_progress_bar=len(texts) > 100,
                convert_to_numpy=True,
                normalize_embeddings=True  # Normalize for cosine similarity
            )
            
            # Convert to list of lists
            return embeddings.tolist()
        
        except Exception as e:
            logger.error(f"Error generating embeddings: {e}")
            raise
    
    def embed_single(self, text: str) -> List[float]:
        """
        Generate embedding for a single text.
        
        Args:
            text: Text string
            
        Returns:
            Embedding vector
        """
        embeddings = self.embed_texts([text])
        return embeddings[0] if embeddings else []
    
    def embed_chunks(
        self,
        chunks: List[dict],
        text_key: str = 'text'
    ) -> List[dict]:
        """
        Generate embeddings for chunks and add to chunk dictionaries.
        
        Args:
            chunks: List of chunk dictionaries
            text_key: Key in chunk dict containing the text
            
        Returns:
            Chunks with 'embedding' field added
        """
        if not chunks:
            return []
        
        # Extract texts
        texts = [chunk[text_key] for chunk in chunks]
        
        # Generate embeddings
        embeddings = self.embed_texts(texts)
        
        # Add embeddings to chunks
        for chunk, embedding in zip(chunks, embeddings):
            chunk['embedding'] = embedding
        
        return chunks
    
    def similarity(self, embedding1: List[float], embedding2: List[float]) -> float:
        """
        Calculate cosine similarity between two embeddings.
        
        Args:
            embedding1: First embedding vector
            embedding2: Second embedding vector
            
        Returns:
            Similarity score between -1 and 1
        """
        vec1 = np.array(embedding1)
        vec2 = np.array(embedding2)
        
        # Cosine similarity
        return float(np.dot(vec1, vec2) / (np.linalg.norm(vec1) * np.linalg.norm(vec2)))


# Global embedder instance (lazy-loaded)
_embedder: Optional[Embedder] = None


def get_embedder() -> Embedder:
    """Get or create global embedder instance."""
    global _embedder
    if _embedder is None:
        _embedder = Embedder()
    return _embedder
