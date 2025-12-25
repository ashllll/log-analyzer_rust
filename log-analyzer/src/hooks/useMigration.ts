/**
 * Migration Hook
 * 
 * Provides utilities for detecting and migrating workspaces from traditional
 * format to CAS format.
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';

export interface MigrationReport {
  workspace_id: string;
  total_files: number;
  migrated_files: number;
  failed_files: number;
  deduplicated_files: number;
  original_size: number;
  cas_size: number;
  failed_file_paths: string[];
  duration_ms: number;
  success: boolean;
}

export interface UseMigrationReturn {
  isMigrating: boolean;
  migrationProgress: MigrationReport | null;
  error: string | null;
  detectWorkspaceFormat: (workspaceId: string) => Promise<'traditional' | 'cas' | 'unknown'>;
  checkNeedsMigration: (workspaceId: string) => Promise<boolean>;
  migrateWorkspace: (workspaceId: string) => Promise<MigrationReport>;
}

/**
 * Hook for workspace migration operations
 */
export function useMigration(): UseMigrationReturn {
  const [isMigrating, setIsMigrating] = useState(false);
  const [migrationProgress, setMigrationProgress] = useState<MigrationReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  /**
   * Detect the format of a workspace
   */
  const detectWorkspaceFormat = useCallback(async (workspaceId: string): Promise<'traditional' | 'cas' | 'unknown'> => {
    try {
      logger.debug('[useMigration] Detecting workspace format:', workspaceId);
      const format = await invoke<string>('detect_workspace_format_cmd', { workspaceId });
      logger.debug('[useMigration] Workspace format:', format);
      return format as 'traditional' | 'cas' | 'unknown';
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logger.error('[useMigration] Failed to detect workspace format:', errorMsg);
      setError(errorMsg);
      return 'unknown';
    }
  }, []);

  /**
   * Check if a workspace needs migration
   */
  const checkNeedsMigration = useCallback(async (workspaceId: string): Promise<boolean> => {
    try {
      logger.debug('[useMigration] Checking if workspace needs migration:', workspaceId);
      const needsMigration = await invoke<boolean>('needs_migration_cmd', { workspaceId });
      logger.debug('[useMigration] Needs migration:', needsMigration);
      return needsMigration;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logger.error('[useMigration] Failed to check migration status:', errorMsg);
      setError(errorMsg);
      return false;
    }
  }, []);

  /**
   * Migrate a workspace from traditional format to CAS format
   */
  const migrateWorkspace = useCallback(async (workspaceId: string): Promise<MigrationReport> => {
    setIsMigrating(true);
    setError(null);
    setMigrationProgress(null);

    try {
      logger.info('[useMigration] Starting workspace migration:', workspaceId);
      
      const report = await invoke<MigrationReport>('migrate_workspace_cmd', { workspaceId });
      
      logger.info('[useMigration] Migration completed:', {
        success: report.success,
        migrated: report.migrated_files,
        failed: report.failed_files,
        deduplicated: report.deduplicated_files,
      });

      setMigrationProgress(report);

      if (!report.success) {
        const errorMsg = `Migration completed with errors: ${report.failed_files} files failed`;
        setError(errorMsg);
        logger.warn('[useMigration]', errorMsg);
      }

      return report;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logger.error('[useMigration] Migration failed:', errorMsg);
      setError(errorMsg);
      throw err;
    } finally {
      setIsMigrating(false);
    }
  }, []);

  return {
    isMigrating,
    migrationProgress,
    error,
    detectWorkspaceFormat,
    checkNeedsMigration,
    migrateWorkspace,
  };
}
