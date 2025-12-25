-- SQLite Schema for Content-Addressable Storage (CAS) Metadata
-- 
-- This schema supports:
-- - File metadata with SHA-256 content addressing
-- - Nested archive tracking with hierarchical relationships
-- - Full-text search using FTS5
-- - Fast lookups via indexes

-- ============================================================================
-- FILES TABLE
-- ============================================================================
-- Stores metadata for all extracted files
-- Each file is identified by its SHA-256 hash (content-addressable)
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Content addressing: SHA-256 hash of file content
    -- This is the key to retrieve actual content from CAS
    sha256_hash TEXT NOT NULL UNIQUE,
    
    -- Virtual path: User-visible logical path
    -- Example: "logs.zip/app/server.log"
    virtual_path TEXT NOT NULL,
    
    -- Original filename (without path)
    original_name TEXT NOT NULL,
    
    -- File size in bytes
    size INTEGER NOT NULL,
    
    -- Last modified time (Unix timestamp)
    modified_time INTEGER NOT NULL,
    
    -- MIME type (e.g., "text/plain", "application/gzip")
    mime_type TEXT,
    
    -- Parent archive ID (NULL for top-level files)
    -- References archives.id for nested tracking
    parent_archive_id INTEGER,
    
    -- Nesting depth (0 = top-level, 1 = inside one archive, etc.)
    depth_level INTEGER NOT NULL DEFAULT 0,
    
    -- Creation timestamp (when added to database)
    created_at INTEGER NOT NULL,
    
    -- Foreign key constraint for nested archive tracking
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- ============================================================================
-- ARCHIVES TABLE
-- ============================================================================
-- Stores metadata for archive files (ZIP, TAR, RAR, etc.)
-- Enables tracking of nested archive hierarchies
CREATE TABLE IF NOT EXISTS archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Content addressing: SHA-256 hash of archive content
    sha256_hash TEXT NOT NULL UNIQUE,
    
    -- Virtual path of the archive
    -- Example: "logs.zip" or "logs.zip/nested.tar.gz"
    virtual_path TEXT NOT NULL,
    
    -- Original archive filename
    original_name TEXT NOT NULL,
    
    -- Archive type: "zip", "tar", "tar.gz", "rar", etc.
    archive_type TEXT NOT NULL,
    
    -- Parent archive ID (NULL for top-level archives)
    parent_archive_id INTEGER,
    
    -- Nesting depth
    depth_level INTEGER NOT NULL DEFAULT 0,
    
    -- Extraction status: "pending", "extracting", "completed", "failed"
    extraction_status TEXT NOT NULL,
    
    -- Creation timestamp
    created_at INTEGER NOT NULL,
    
    -- Foreign key constraint for nested archive tracking
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- ============================================================================
-- INDEXES
-- ============================================================================
-- Optimize common query patterns

-- Fast lookup by virtual path (used in search results)
CREATE INDEX IF NOT EXISTS idx_files_virtual_path ON files(virtual_path);

-- Fast lookup of files within an archive (nested tracking)
CREATE INDEX IF NOT EXISTS idx_files_parent_archive ON files(parent_archive_id);

-- Fast lookup by SHA-256 hash (content retrieval)
CREATE INDEX IF NOT EXISTS idx_files_hash ON files(sha256_hash);

-- Fast lookup of archives by virtual path
CREATE INDEX IF NOT EXISTS idx_archives_virtual_path ON archives(virtual_path);

-- Fast lookup of nested archives
CREATE INDEX IF NOT EXISTS idx_archives_parent ON archives(parent_archive_id);

-- Fast lookup of archives by hash
CREATE INDEX IF NOT EXISTS idx_archives_hash ON archives(sha256_hash);

-- Fast lookup by depth level (useful for limiting nesting)
CREATE INDEX IF NOT EXISTS idx_files_depth ON files(depth_level);
CREATE INDEX IF NOT EXISTS idx_archives_depth ON archives(depth_level);

-- ============================================================================
-- FULL-TEXT SEARCH (FTS5)
-- ============================================================================
-- Virtual table for fast full-text search on file paths and names
-- Uses SQLite's FTS5 extension for efficient text search
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    virtual_path,
    original_name,
    content='files',
    content_rowid='id'
);

-- Triggers to keep FTS index in sync with files table
CREATE TRIGGER IF NOT EXISTS files_fts_insert AFTER INSERT ON files BEGIN
    INSERT INTO files_fts(rowid, virtual_path, original_name)
    VALUES (new.id, new.virtual_path, new.original_name);
END;

CREATE TRIGGER IF NOT EXISTS files_fts_delete AFTER DELETE ON files BEGIN
    DELETE FROM files_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS files_fts_update AFTER UPDATE ON files BEGIN
    DELETE FROM files_fts WHERE rowid = old.id;
    INSERT INTO files_fts(rowid, virtual_path, original_name)
    VALUES (new.id, new.virtual_path, new.original_name);
END;

-- ============================================================================
-- VIEWS
-- ============================================================================
-- Useful views for common queries

-- View: All files with their archive information
CREATE VIEW IF NOT EXISTS v_files_with_archives AS
SELECT 
    f.id,
    f.sha256_hash,
    f.virtual_path,
    f.original_name,
    f.size,
    f.modified_time,
    f.mime_type,
    f.depth_level,
    a.virtual_path as archive_path,
    a.archive_type,
    a.extraction_status
FROM files f
LEFT JOIN archives a ON f.parent_archive_id = a.id;

-- View: Archive hierarchy (nested structure)
CREATE VIEW IF NOT EXISTS v_archive_hierarchy AS
WITH RECURSIVE archive_tree AS (
    -- Base case: top-level archives
    SELECT 
        id,
        sha256_hash,
        virtual_path,
        original_name,
        archive_type,
        parent_archive_id,
        depth_level,
        virtual_path as full_path
    FROM archives
    WHERE parent_archive_id IS NULL
    
    UNION ALL
    
    -- Recursive case: nested archives
    SELECT 
        a.id,
        a.sha256_hash,
        a.virtual_path,
        a.original_name,
        a.archive_type,
        a.parent_archive_id,
        a.depth_level,
        at.full_path || '/' || a.virtual_path as full_path
    FROM archives a
    INNER JOIN archive_tree at ON a.parent_archive_id = at.id
)
SELECT * FROM archive_tree;

-- ============================================================================
-- STATISTICS QUERIES
-- ============================================================================
-- Pre-defined queries for common statistics

-- Total file count
-- SELECT COUNT(*) FROM files;

-- Total archive count
-- SELECT COUNT(*) FROM archives;

-- Total storage size
-- SELECT SUM(size) FROM files;

-- Maximum nesting depth
-- SELECT MAX(depth_level) FROM files;

-- Files by archive
-- SELECT a.virtual_path, COUNT(f.id) as file_count
-- FROM archives a
-- LEFT JOIN files f ON f.parent_archive_id = a.id
-- GROUP BY a.id;

-- Deduplication ratio (files with same hash)
-- SELECT sha256_hash, COUNT(*) as duplicate_count
-- FROM files
-- GROUP BY sha256_hash
-- HAVING COUNT(*) > 1;
