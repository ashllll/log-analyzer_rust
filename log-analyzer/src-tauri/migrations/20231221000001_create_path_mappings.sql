-- Create path_mappings table for storing shortened path mappings
-- This table maintains bidirectional mappings between original and shortened paths
-- to support long filename handling and path length management

CREATE TABLE IF NOT EXISTS path_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    short_path TEXT NOT NULL,
    original_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    access_count INTEGER DEFAULT 0,
    UNIQUE(workspace_id, short_path)
);

-- Index for fast lookups by workspace_id and short_path (primary query pattern)
CREATE INDEX IF NOT EXISTS idx_workspace_short 
ON path_mappings(workspace_id, short_path);

-- Index for reverse lookups by workspace_id and original_path
CREATE INDEX IF NOT EXISTS idx_workspace_original 
ON path_mappings(workspace_id, original_path);

-- Index for cleanup operations by workspace_id
CREATE INDEX IF NOT EXISTS idx_workspace_id 
ON path_mappings(workspace_id);
