"""REST API routes for Legal MCP Server."""

import time
from datetime import datetime, timedelta
from typing import List
from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import desc, select
from sqlalchemy.ext.asyncio import AsyncSession

from src.api.schemas import (
    CaseLawResponse,
    ErrorResponse,
    HealthResponse,
    LawResponse,
    LegalChangeItem,
    LegalChangesRequest,
    LegalChangesResponse,
    RelatedLawItem,
    RelatedLawsRequest,
    RelatedLawsResponse,
    SearchCaseLawRequest,
    SearchLawsRequest,
    SearchLawsResponse,
    SearchResultItem,
)
from src.config import get_settings
from src.db.connection import get_db
from src.db.models import CaseLaw, Law
from src.search.embeddings import get_embedding_service
from src.search.vector_store import VectorStore

settings = get_settings()
router = APIRouter()


@router.get("/health", response_model=HealthResponse)
async def health_check(db: AsyncSession = Depends(get_db)):
    """
    Health check endpoint.

    Returns service status and component health.
    """
    # Check database
    db_status = "healthy"
    try:
        await db.execute(select(1))
    except Exception as e:
        db_status = f"unhealthy: {str(e)}"

    # Check ML model
    model_status = "healthy"
    try:
        embedding_service = get_embedding_service()
        _ = embedding_service._load_model()
    except Exception as e:
        model_status = f"unhealthy: {str(e)}"

    return HealthResponse(
        status="healthy" if db_status == "healthy" and model_status == "healthy" else "degraded",
        version=settings.app_version,
        database=db_status,
        model=model_status,
    )


@router.post("/search", response_model=SearchLawsResponse)
async def search_laws(
    request: SearchLawsRequest,
    db: AsyncSession = Depends(get_db),
):
    """
    Search for laws using semantic similarity.

    Args:
        request: Search request parameters
        db: Database session

    Returns:
        Search results with similarity scores
    """
    start_time = time.time()

    # Create vector store
    vector_store = VectorStore(db)

    # Perform search
    results = await vector_store.search_laws(
        query=request.query,
        limit=request.limit,
        min_score=request.min_score or settings.min_similarity_score,
        jurisdiction_filter=[j.value for j in request.jurisdiction] if request.jurisdiction else None,
        document_type_filter=[d.value for d in request.document_type] if request.document_type else None,
    )

    # Fetch full law details for each result
    search_items: List[SearchResultItem] = []
    for result in results:
        # Get law details
        stmt = select(Law).where(Law.id == result["law_id"])
        law_result = await db.execute(stmt)
        law = law_result.scalar_one_or_none()

        if law:
            search_items.append(
                SearchResultItem(
                    id=law.id,
                    type="law",
                    title=law.title,
                    content=result["content"],
                    similarity=result["similarity"],
                    jurisdiction=law.jurisdiction,
                    metadata={
                        "abbreviation": law.abbreviation,
                        "document_type": law.document_type.value,
                        "section_reference": result.get("section_reference"),
                        "source_url": law.source_url,
                    },
                )
            )

    # If include_case_law is True, also search case law
    if request.include_case_law:
        case_law_results = await vector_store.search_case_law(
            query=request.query,
            limit=request.limit // 2,  # Reserve half the results for case law
            min_score=request.min_score or settings.min_similarity_score,
            jurisdiction_filter=[j.value for j in request.jurisdiction] if request.jurisdiction else None,
        )

        for result in case_law_results:
            # Get case law details
            stmt = select(CaseLaw).where(CaseLaw.id == result["case_law_id"])
            case_result = await db.execute(stmt)
            case_law = case_result.scalar_one_or_none()

            if case_law:
                search_items.append(
                    SearchResultItem(
                        id=case_law.id,
                        type="case_law",
                        title=case_law.title or f"{case_law.court} - {case_law.case_number}",
                        content=result["content"],
                        similarity=result["similarity"],
                        jurisdiction=case_law.jurisdiction,
                        metadata={
                            "court": case_law.court,
                            "case_number": case_law.case_number,
                            "decision_date": case_law.decision_date.isoformat(),
                            "source_url": case_law.source_url,
                        },
                    )
                )

    # Sort by similarity and limit
    search_items.sort(key=lambda x: x.similarity, reverse=True)
    search_items = search_items[: request.limit]

    execution_time = (time.time() - start_time) * 1000

    return SearchLawsResponse(
        query=request.query,
        results=search_items,
        total=len(search_items),
        execution_time_ms=execution_time,
    )


@router.get("/laws/{law_id}", response_model=LawResponse)
async def get_law_by_id(
    law_id: UUID,
    db: AsyncSession = Depends(get_db),
):
    """
    Get a law by its UUID.

    Args:
        law_id: UUID of the law
        db: Database session

    Returns:
        Law details

    Raises:
        HTTPException: If law not found
    """
    stmt = select(Law).where(Law.id == law_id)
    result = await db.execute(stmt)
    law = result.scalar_one_or_none()

    if not law:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Law with ID {law_id} not found",
        )

    return LawResponse.model_validate(law)


@router.get("/laws/abbreviation/{abbreviation}", response_model=LawResponse)
async def get_law_by_abbreviation(
    abbreviation: str,
    db: AsyncSession = Depends(get_db),
):
    """
    Get a law by its abbreviation.

    Args:
        abbreviation: Law abbreviation (e.g., 'BGB', 'StGB')
        db: Database session

    Returns:
        Law details

    Raises:
        HTTPException: If law not found
    """
    stmt = select(Law).where(Law.abbreviation.ilike(abbreviation))
    result = await db.execute(stmt)
    law = result.scalar_one_or_none()

    if not law:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Law with abbreviation '{abbreviation}' not found",
        )

    return LawResponse.model_validate(law)


@router.post("/search/case-law", response_model=SearchLawsResponse)
async def search_case_law(
    request: SearchCaseLawRequest,
    db: AsyncSession = Depends(get_db),
):
    """
    Search for case law using semantic similarity.

    Args:
        request: Search request parameters
        db: Database session

    Returns:
        Search results with similarity scores
    """
    start_time = time.time()

    vector_store = VectorStore(db)

    results = await vector_store.search_case_law(
        query=request.query,
        limit=request.limit,
        min_score=request.min_score or settings.min_similarity_score,
        jurisdiction_filter=[j.value for j in request.jurisdiction] if request.jurisdiction else None,
        court_level_filter=[c.value for c in request.court_level] if request.court_level else None,
    )

    # Fetch full case law details
    search_items: List[SearchResultItem] = []
    for result in results:
        stmt = select(CaseLaw).where(CaseLaw.id == result["case_law_id"])
        case_result = await db.execute(stmt)
        case_law = case_result.scalar_one_or_none()

        if case_law:
            search_items.append(
                SearchResultItem(
                    id=case_law.id,
                    type="case_law",
                    title=case_law.title or f"{case_law.court} - {case_law.case_number}",
                    content=result["content"],
                    similarity=result["similarity"],
                    jurisdiction=case_law.jurisdiction,
                    metadata={
                        "court": case_law.court,
                        "case_number": case_law.case_number,
                        "decision_date": case_law.decision_date.isoformat(),
                        "source_url": case_law.source_url,
                    },
                )
            )

    execution_time = (time.time() - start_time) * 1000

    return SearchLawsResponse(
        query=request.query,
        results=search_items,
        total=len(search_items),
        execution_time_ms=execution_time,
    )


@router.post("/changes", response_model=LegalChangesResponse)
async def get_legal_changes(
    request: LegalChangesRequest,
    db: AsyncSession = Depends(get_db),
):
    """
    Get recent legal changes (new laws, modifications).

    Args:
        request: Request parameters
        db: Database session

    Returns:
        List of recent legal changes
    """
    cutoff_date = datetime.now().date() - timedelta(days=request.days)

    # Query recently modified or created laws
    stmt = select(Law).where(
        (Law.last_modified_date >= cutoff_date) | (Law.publication_date >= cutoff_date)
    )

    if request.jurisdiction:
        stmt = stmt.where(Law.jurisdiction.in_([j.value for j in request.jurisdiction]))

    stmt = stmt.order_by(desc(Law.last_modified_date)).limit(request.limit)

    result = await db.execute(stmt)
    laws = result.scalars().all()

    changes: List[LegalChangeItem] = []
    for law in laws:
        # Determine change type
        change_type = "modified"
        change_date = law.last_modified_date or law.publication_date

        if law.publication_date and law.publication_date >= cutoff_date:
            change_type = "new"
            change_date = law.publication_date

        if law.valid_until and law.valid_until < datetime.now().date():
            change_type = "repealed"

        changes.append(
            LegalChangeItem(
                id=law.id,
                title=law.title,
                abbreviation=law.abbreviation,
                jurisdiction=law.jurisdiction,
                document_type=law.document_type,
                change_date=change_date,
                change_type=change_type,
                source_url=law.source_url,
            )
        )

    return LegalChangesResponse(
        changes=changes,
        total=len(changes),
        days=request.days,
    )


@router.post("/related", response_model=RelatedLawsResponse)
async def get_related_laws(
    request: RelatedLawsRequest,
    db: AsyncSession = Depends(get_db),
):
    """
    Get laws related to a specific law.

    Args:
        request: Request parameters
        db: Database session

    Returns:
        List of related laws

    Raises:
        HTTPException: If law not found
    """
    # Check if law exists
    stmt = select(Law).where(Law.id == request.law_id)
    result = await db.execute(stmt)
    law = result.scalar_one_or_none()

    if not law:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Law with ID {request.law_id} not found",
        )

    # Find similar laws
    vector_store = VectorStore(db)
    similar_chunks = await vector_store.get_similar_laws(
        law_id=request.law_id,
        limit=request.limit,
    )

    # Get unique laws from chunks
    related_law_ids = list({chunk["law_id"] for chunk in similar_chunks})

    # Fetch law details
    related_items: List[RelatedLawItem] = []
    for law_id in related_law_ids:
        stmt = select(Law).where(Law.id == law_id)
        result = await db.execute(stmt)
        related_law = result.scalar_one_or_none()

        if related_law:
            # Get the best similarity score for this law
            best_similarity = max(
                chunk["similarity"]
                for chunk in similar_chunks
                if chunk["law_id"] == law_id
            )

            related_items.append(
                RelatedLawItem(
                    id=related_law.id,
                    title=related_law.title,
                    abbreviation=related_law.abbreviation,
                    jurisdiction=related_law.jurisdiction,
                    document_type=related_law.document_type,
                    similarity=best_similarity,
                    relation_type=None,  # Could be populated from explicit relations
                )
            )

    # Sort by similarity
    related_items.sort(key=lambda x: x.similarity, reverse=True)

    return RelatedLawsResponse(
        law_id=request.law_id,
        related_laws=related_items[: request.limit],
        total=len(related_items),
    )


@router.get("/case-law/{case_law_id}", response_model=CaseLawResponse)
async def get_case_law_by_id(
    case_law_id: UUID,
    db: AsyncSession = Depends(get_db),
):
    """
    Get case law by its UUID.

    Args:
        case_law_id: UUID of the case law
        db: Database session

    Returns:
        Case law details

    Raises:
        HTTPException: If case law not found
    """
    stmt = select(CaseLaw).where(CaseLaw.id == case_law_id)
    result = await db.execute(stmt)
    case_law = result.scalar_one_or_none()

    if not case_law:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Case law with ID {case_law_id} not found",
        )

    return CaseLawResponse.model_validate(case_law)


@router.post("/chat", response_model=dict)
async def chat_with_gemini(
    request: dict,
    db: AsyncSession = Depends(get_db),
):
    """
    Chat with Gemini using RAG over legal documents.
    
    Args:
        request: Chat request with message, optional context search
        db: Database session
        
    Returns:
        Chat response with answer and sources
    """
    try:
        from src.api.gemini import get_gemini_service, ChatRequest
        from src.search.vector_store import VectorStore
        
        message = request.get("message", "")
        if not message:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Message is required"
            )
        
        # Auto-search for relevant context if not provided
        context = request.get("context", [])
        if not context:
            # Use vector search to find relevant law chunks
            from src.db.models import Law
            
            # Initialize vector store
            vector_store = VectorStore(db)
            
            # Search for relevant chunks using semantic similarity
            search_results = await vector_store.search_laws(
                query=message,
                limit=5,  # Get top 5 most relevant chunks
                min_score=0.3  # Minimum similarity threshold
            )
            
            # Get law information for each chunk
            law_cache = {}
            for result in search_results:
                law_id = result["law_id"]
                
                # Cache law info to avoid repeated queries
                if law_id not in law_cache:
                    stmt = select(Law).where(Law.id == law_id)
                    law_result = await db.execute(stmt)
                    law = law_result.scalar_one_or_none()
                    if law:
                        law_cache[law_id] = law
                
                law = law_cache.get(law_id)
                if law:
                    context.append({
                        "title": law.title,
                        "content": result["content"],
                        "jurisdiction": law.jurisdiction.value,
                        "similarity": result["similarity"],
                        "metadata": {
                            "abbreviation": law.abbreviation,
                            "section_reference": result["section_reference"],
                            "source_url": law.source_url,
                            "document_type": law.document_type.value
                        }
                    })
        
        # Build chat request
        history = request.get("history", [])
        chat_request = ChatRequest(
            message=message,
            context=context,
            history=history
        )
        
        # Get Gemini service
        gemini_service = get_gemini_service()
        
        # Get response
        response = await gemini_service.chat(chat_request)
        
        return {
            "answer": response.answer,
            "sources": response.sources,
            "model_used": response.model_used,
            "context_size": len(context)
        }
        
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Chat error: {str(e)}"
        )


@router.post("/admin/generate-embeddings")
async def generate_embeddings(
    db: AsyncSession = Depends(get_db),
):
    """
    Generate embeddings for all laws in the database.
    This creates law_chunks with vector embeddings for semantic search.
    
    Returns:
        Summary of chunks created
    """
    try:
        from src.db.models import LawChunk
        from uuid import uuid4
        
        # Get all laws
        stmt = select(Law)
        result = await db.execute(stmt)
        laws = result.scalars().all()
        
        if not laws:
            return {"message": "No laws found in database", "chunks_created": 0}
        
        embedding_service = get_embedding_service()
        total_chunks = 0
        laws_processed = []
        
        for law in laws:
            if not law.full_text:
                continue
            
            # Split text into meaningful parts (by double newline - usually paragraphs/articles)
            text_parts = [p.strip() for p in law.full_text.split('\n\n') if p.strip() and len(p.strip()) > 50]
            
            chunks_for_law = 0
            
            for idx, part in enumerate(text_parts):
                # Extract section reference if it starts with Artikel/Article
                section_ref = None
                if part.startswith(('Artikel', 'Article', 'Art.')):
                    first_line = part.split('\n')[0]
                    section_ref = first_line.split('-')[0].strip()
                
                # Generate embedding
                try:
                    embedding = await embedding_service.encode_text(part)
                    
                    # Create chunk
                    chunk = LawChunk(
                        id=uuid4(),
                        law_id=law.id,
                        chunk_index=idx,
                        section_reference=section_ref,
                        content=part[:5000],  # Limit content length to avoid huge DB entries
                        embedding=embedding,
                        metadata={}
                    )
                    
                    db.add(chunk)
                    chunks_for_law += 1
                    
                except Exception as e:
                    # Log error but continue with other chunks
                    print(f"Error creating chunk for {law.abbreviation}: {e}")
                    continue
            
            # Commit after each law
            await db.commit()
            total_chunks += chunks_for_law
            
            laws_processed.append({
                "law": law.abbreviation or law.title[:50],
                "chunks_created": chunks_for_law
            })
        
        return {
            "message": "Embeddings generated successfully",
            "laws_processed": len(laws_processed),
            "total_chunks_created": total_chunks,
            "details": laws_processed
        }
        
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error generating embeddings: {str(e)}"
        )
