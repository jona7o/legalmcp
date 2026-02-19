-- Install pgvector extension
-- This will fail silently if pgvector is not available
-- In that case, we'll use ARRAY instead

-- Try to create the extension, but don't fail if it doesn't exist
DO $$
BEGIN
    CREATE EXTENSION IF NOT EXISTS vector;
EXCEPTION
    WHEN undefined_file THEN
        RAISE NOTICE 'pgvector extension not available, will use workaround';
END
$$;
