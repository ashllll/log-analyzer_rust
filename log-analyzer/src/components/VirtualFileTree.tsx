import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChevronRight, ChevronDown, File, Archive, Loader2, AlertCircle } from 'lucide-react';
import { cn } from '../utils/classNames';
import { logger } from '../utils/logger';

/**
 * Virtual File Tree Node Types
 */
interface FileNode {
  type: 'file';
  name: string;
  path: string;
  hash: string;
  size: number;
  mimeType?: string;
}

interface ArchiveNode {
  type: 'archive';
  name: string;
  path: string;
  hash: string;
  archiveType: string;
  children: TreeNode[];
}

type TreeNode = FileNode | ArchiveNode;

/**
 * Component Props
 */
interface VirtualFileTreeProps {
  workspaceId: string;
  onFileSelect?: (hash: string, path: string) => void;
  className?: string;
}

/**
 * Tree Node Component Props
 */
interface TreeNodeComponentProps {
  node: TreeNode;
  level: number;
  onFileSelect?: (hash: string, path: string) => void;
}

/**
 * Format file size for display
 */
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

/**
 * Tree Node Component
 * 
 * Renders a single node in the tree with expand/collapse functionality
 * for archives and click handling for files.
 */
const TreeNodeComponent: React.FC<TreeNodeComponentProps> = ({ 
  node, 
  level, 
  onFileSelect 
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const indentPx = level * 16;

  const handleToggle = useCallback(() => {
    if (node.type === 'archive') {
      setIsExpanded(prev => !prev);
    }
  }, [node.type]);

  const handleFileClick = useCallback(() => {
    if (node.type === 'file' && onFileSelect) {
      onFileSelect(node.hash, node.path);
    }
  }, [node, onFileSelect]);

  if (node.type === 'file') {
    return (
      <div
        style={{ paddingLeft: `${indentPx}px` }}
        className={cn(
          "flex items-center gap-2 px-2 py-1 hover:bg-bg-hover cursor-pointer",
          "border-b border-border-base/30 transition-colors"
        )}
        onClick={handleFileClick}
      >
        <File size={14} className="text-blue-400 shrink-0" />
        <span className="text-xs text-text-main truncate flex-1" title={node.name}>
          {node.name}
        </span>
        <span className="text-[10px] text-text-dim shrink-0">
          {formatFileSize(node.size)}
        </span>
      </div>
    );
  }

  // Archive node
  return (
    <div>
      <div
        style={{ paddingLeft: `${indentPx}px` }}
        className={cn(
          "flex items-center gap-2 px-2 py-1 hover:bg-bg-hover cursor-pointer",
          "border-b border-border-base/30 transition-colors"
        )}
        onClick={handleToggle}
      >
        {isExpanded ? (
          <ChevronDown size={14} className="text-text-dim shrink-0" />
        ) : (
          <ChevronRight size={14} className="text-text-dim shrink-0" />
        )}
        <Archive size={14} className="text-yellow-400 shrink-0" />
        <span className="text-xs text-text-main truncate flex-1" title={node.name}>
          {node.name}
        </span>
        <span className="text-[10px] text-text-dim shrink-0">
          {node.archiveType.toUpperCase()}
        </span>
      </div>
      {isExpanded && node.children.length > 0 && (
        <div>
          {node.children.map((child, index) => (
            <TreeNodeComponent
              key={`${child.path}-${index}`}
              node={child}
              level={level + 1}
              onFileSelect={onFileSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * Virtual File Tree Component
 * 
 * Displays the hierarchical structure of files and nested archives
 * in a workspace. Supports expand/collapse for archives and file
 * selection callbacks.
 * 
 * # Requirements
 * 
 * Validates: Requirements 4.2
 * 
 * # Features
 * 
 * - Hierarchical display of files and archives
 * - Expand/collapse functionality for archives
 * - File size display
 * - Archive type indicators
 * - Click handling for file selection
 * - Loading and error states
 * 
 * # Example
 * 
 * ```tsx
 * <VirtualFileTree
 *   workspaceId="workspace_123"
 *   onFileSelect={(hash, path) => {
 *     console.log('Selected file:', path, 'with hash:', hash);
 *   }}
 * />
 * ```
 */
const VirtualFileTree: React.FC<VirtualFileTreeProps> = ({
  workspaceId,
  onFileSelect,
  className
}) => {
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load tree structure when workspace changes
  useEffect(() => {
    if (!workspaceId) {
      setTree([]);
      setIsLoading(false);
      return;
    }

    const loadTree = async () => {
      setIsLoading(true);
      setError(null);

      try {
        logger.debug('Loading virtual file tree for workspace:', workspaceId);
        
        const treeData = await invoke<TreeNode[]>('get_virtual_file_tree', {
          workspaceId
        });

        logger.debug('Loaded tree with', treeData.length, 'root nodes');
        setTree(treeData);
      } catch (err) {
        const errorMsg = err && err instanceof Error ? err.message : String(err || 'Unknown error');
        logger.error('Failed to load virtual file tree:', errorMsg);
        setError(errorMsg);
      } finally {
        setIsLoading(false);
      }
    };

    loadTree();
  }, [workspaceId]);

  // Loading state
  if (isLoading) {
    return (
      <div className={cn("flex items-center justify-center p-8", className)}>
        <Loader2 size={24} className="animate-spin text-primary" />
        <span className="ml-2 text-sm text-text-dim">Loading file tree...</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className={cn("flex items-center justify-center p-8", className)}>
        <AlertCircle size={24} className="text-red-400" />
        <div className="ml-2">
          <div className="text-sm font-semibold text-red-400">Failed to load file tree</div>
          <div className="text-xs text-text-dim mt-1">{error}</div>
        </div>
      </div>
    );
  }

  // Empty state
  if (tree.length === 0) {
    return (
      <div className={cn("flex items-center justify-center p-8 text-text-dim", className)}>
        <span className="text-sm">No files in workspace</span>
      </div>
    );
  }

  // Tree display
  return (
    <div className={cn("overflow-auto bg-bg-main", className)}>
      <div className="min-w-full">
        {tree.map((node, index) => (
          <TreeNodeComponent
            key={`${node.path}-${index}`}
            node={node}
            level={0}
            onFileSelect={onFileSelect}
          />
        ))}
      </div>
    </div>
  );
};

export default VirtualFileTree;
