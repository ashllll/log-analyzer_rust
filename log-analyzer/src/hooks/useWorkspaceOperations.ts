/**
 * 工作区操作Hook
 *
 * @deprecated 请使用分解后的专用Hook:
 * - useWorkspaceImport: 导入操作 (importPath, importFolder, importFile)
 * - useWorkspaceManagement: 管理操作 (deleteWorkspace, refreshWorkspace)
 * - useWorkspaceSelection: 选择操作 (switchWorkspace, activeWorkspace)
 * - useWorkspaceWatch: 监听操作 (toggleWatch)
 * - useWorkspaceList: 列表操作 (refreshWorkspaces)
 *
 * 此文件保留用于向后兼容，逐步迁移到专用Hook
 */

import { useWorkspaceImport } from './useWorkspaceImport';
import { useWorkspaceManagement } from './useWorkspaceManagement';
import { useWorkspaceSelection } from './useWorkspaceSelection';
import { useWorkspaceWatch } from './useWorkspaceWatch';
import { useWorkspaceList } from './useWorkspaceList';

/**
 * 工作区操作Hook（组合版）
 * 封装所有工作区相关的后端操作
 * 统一错误处理和加载状态管理
 */
export const useWorkspaceOperations = () => {
  const { importPath, importFolder, importFile, isLoading: importLoading } = useWorkspaceImport();
  const { deleteWorkspace, refreshWorkspace, isLoading: managementLoading } = useWorkspaceManagement();
  const { switchWorkspace, activeWorkspaceId, activeWorkspace, workspaces, isPending } = useWorkspaceSelection();
  const { toggleWatch, isLoading: watchLoading } = useWorkspaceWatch();
  const { refreshWorkspaces, loading: listLoading, error } = useWorkspaceList();

  const loading = importLoading || managementLoading || watchLoading || listLoading || isPending;

  return {
    // 状态
    workspaces,
    activeWorkspace,
    activeWorkspaceId,
    loading,
    error,

    // 导入操作
    importPath,
    importFolder,
    importFile,

    // 管理操作
    refreshWorkspace,
    deleteWorkspace,

    // 选择操作
    switchWorkspace,

    // 监听操作
    toggleWatch,

    // 列表操作
    refreshWorkspaces,
  };
};
