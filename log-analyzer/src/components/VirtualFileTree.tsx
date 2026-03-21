import React, { useState, useEffect, useCallback, useRef, memo, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useVirtualizer } from '@tanstack/react-virtual';
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
 * 扁平化的树节点，用于虚拟滚动
 */
interface FlatTreeNode {
  node: TreeNode;
  level: number;
  isVisible: boolean;
  key: string;
}

/**
 * Component Props
 */
interface VirtualFileTreeProps {
  workspaceId: string;
  onFileSelect?: (hash: string, path: string) => void;
  className?: string;
}

/**
 * Format file size for display
 * 
 * # Safety
 * - Handles bytes=0 case
 * - Handles very large numbers (PB+ range)
 * - Prevents array index out of bounds
 */
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  if (bytes < 0) return 'Invalid size';
  
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  // Limit index to prevent array out of bounds
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(k)),
    sizes.length - 1
  );
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

/**
 * 展开状态管理 - 使用 Map 存储每个节点的展开状态
 */
type ExpandedState = Map<string, boolean>;

/**
 * 扁平化树结构为单层数组，用于虚拟滚动
 */
function flattenTree(
  nodes: TreeNode[],
  expandedState: ExpandedState,
  level = 0
): FlatTreeNode[] {
  const result: FlatTreeNode[] = [];

  for (const node of nodes) {
    const key = node.path;
    const isExpanded = expandedState.get(key) ?? false;

    result.push({
      node,
      level,
      isVisible: true, // 简化：根节点总是可见
      key
    });

    // 如果是压缩包且已展开，递归处理子节点
    if (node.type === 'archive' && isExpanded && node.children.length > 0) {
      result.push(...flattenTree(node.children, expandedState, level + 1));
    }
  }

  return result;
}

/**
 * 树节点行组件 - 使用 React.memo 优化
 */
interface VirtualTreeNodeRowProps {
  flatNode: FlatTreeNode;
  onFileSelect?: (hash: string, path: string) => void;
  onToggle: (path: string) => void;
  isExpanded: boolean;
  virtualStart: number;
  virtualKey: React.Key;
  measureRef: (node: Element | null) => void;
}

const VirtualTreeNodeRow = memo<VirtualTreeNodeRowProps>(({
  flatNode,
  onFileSelect,
  onToggle,
  isExpanded,
  virtualStart,
  virtualKey,
  measureRef
}) => {
  const { node, level } = flatNode;
  const indentPx = level * 16;

  // Hooks must be called before any conditional returns
  const handleFileClick = useCallback(() => {
    if (onFileSelect) {
      onFileSelect(node.hash, node.path);
    }
  }, [node.hash, node.path, onFileSelect]);

  const handleArchiveToggle = useCallback(() => {
    onToggle(node.path);
  }, [node.path, onToggle]);

  if (node.type === 'file') {
    return (
      <div
        ref={measureRef}
        key={virtualKey}
        data-index={virtualKey}
        style={{
          transform: `translateY(${virtualStart}px)`,
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%'
        }}
      >
        <div
          style={{ paddingLeft: `${indentPx}px` }}
          className={cn(
            "flex items-center gap-2 px-2 py-1.5 hover:bg-bg-hover cursor-pointer",
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
      </div>
    );
  }

  // Archive node
  return (
    <div
      ref={measureRef}
      key={virtualKey}
      data-index={virtualKey}
      style={{
        transform: `translateY(${virtualStart}px)`,
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%'
      }}
    >
      <div
        style={{ paddingLeft: `${indentPx}px` }}
        className={cn(
          "flex items-center gap-2 px-2 py-1.5 hover:bg-bg-hover cursor-pointer",
          "border-b border-border-base/30 transition-colors"
        )}
        onClick={handleArchiveToggle}
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
    </div>
  );
}, (prevProps, nextProps) => {
  return (
    prevProps.flatNode.key === nextProps.flatNode.key &&
    prevProps.isExpanded === nextProps.isExpanded &&
    prevProps.virtualStart === nextProps.virtualStart
  );
});

VirtualTreeNodeRow.displayName = 'VirtualTreeNodeRow';

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
 * - **虚拟滚动支持**: 使用 @tanstack/react-virtual 实现高性能渲染
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

  // 展开状态管理
  const [expandedState, setExpandedState] = useState<ExpandedState>(new Map());

  const parentRef = useRef<HTMLDivElement>(null);

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

        // 默认展开第一层
        const initialExpanded = new Map<string, boolean>();
        treeData.forEach(node => {
          if (node.type === 'archive') {
            initialExpanded.set(node.path, false); // 默认折叠
          }
        });
        setExpandedState(initialExpanded);
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

  // 扁平化树结构 - 使用 useMemo 优化
  const flatTree = useMemo(() => {
    return flattenTree(tree, expandedState);
  }, [tree, expandedState]);

  // 切换节点展开状态
  const handleToggle = useCallback((path: string) => {
    setExpandedState(prev => {
      const next = new Map(prev);
      next.set(path, !next.get(path));
      return next;
    });
  }, []);

  // 虚拟滚动配置
  const rowVirtualizer = useVirtualizer({
    count: flatTree.length,
    getScrollElement: () => parentRef.current,
    estimateSize: useCallback(() => 36, []), // 每行高度约 36px
    overscan: 10,
    // 启用调试模式以便在测试环境中正常工作
    debug: false,
  });

  // 获取虚拟滚动项目，如果没有则使用全部项目（fallback）
  const virtualItems = rowVirtualizer.getVirtualItems();
  const useVirtualScrolling = virtualItems.length > 0 || flatTree.length > 100;

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

  // Tree display with virtual scrolling
  return (
    <div
      ref={parentRef}
      className={cn("overflow-auto bg-bg-main", className)}
      style={{ height: '100%', minHeight: 400 }}
    >
      {useVirtualScrolling ? (
        // 虚拟滚动模式
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: '100%',
            position: 'relative'
          }}
        >
          {virtualItems.map((virtualRow) => {
            const flatNode = flatTree[virtualRow.index];
            if (!flatNode) return null;

            // 获取展开状态（仅对 archive 节点有效）
            const isExpanded = flatNode.node.type === 'archive'
              ? (expandedState.get(flatNode.node.path) ?? false)
              : false;

            return (
              <VirtualTreeNodeRow
                key={virtualRow.key}
                flatNode={flatNode}
                onFileSelect={onFileSelect}
                onToggle={handleToggle}
                isExpanded={isExpanded}
                virtualStart={virtualRow.start}
                virtualKey={virtualRow.key}
                measureRef={rowVirtualizer.measureElement}
              />
            );
          })}
        </div>
      ) : (
        // Fallback: 直接渲染所有项目（用于测试环境或小数据集）
        flatTree.map((flatNode) => {
          const isExpanded = flatNode.node.type === 'archive'
            ? (expandedState.get(flatNode.node.path) ?? false)
            : false;

          return (
            <div
              key={flatNode.key}
              style={{ paddingLeft: `${flatNode.level * 16}px` }}
              className={cn(
                "flex items-center gap-2 px-2 py-1.5 hover:bg-bg-hover cursor-pointer",
                "border-b border-border-base/30 transition-colors"
              )}
              onClick={() => {
                if (flatNode.node.type === 'archive') {
                  handleToggle(flatNode.node.path);
                } else if (onFileSelect) {
                  onFileSelect(flatNode.node.hash, flatNode.node.path);
                }
              }}
            >
              {flatNode.node.type === 'archive' && (
                isExpanded ? (
                  <ChevronDown size={14} className="text-text-dim shrink-0" />
                ) : (
                  <ChevronRight size={14} className="text-text-dim shrink-0" />
                )
              )}
              {flatNode.node.type === 'file' ? (
                <File size={14} className="text-blue-400 shrink-0" />
              ) : (
                <Archive size={14} className="text-yellow-400 shrink-0" />
              )}
              <span className="text-xs text-text-main truncate flex-1" title={flatNode.node.name}>
                {flatNode.node.name}
              </span>
              {flatNode.node.type === 'file' ? (
                <span className="text-[10px] text-text-dim shrink-0">
                  {formatFileSize(flatNode.node.size)}
                </span>
              ) : (
                <span className="text-[10px] text-text-dim shrink-0">
                  {flatNode.node.archiveType.toUpperCase()}
                </span>
              )}
            </div>
          );
        })
      )}
    </div>
  );
};

export default VirtualFileTree;
