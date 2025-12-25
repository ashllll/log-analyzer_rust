/**
 * Migration Dialog Component
 * 
 * Displays a dialog prompting users to migrate their workspace from traditional
 * format to the new CAS format.
 */

import { useState } from 'react';
import { Button } from './ui';
import { useMigration, type MigrationReport } from '../hooks/useMigration';
import { logger } from '../utils/logger';

interface MigrationDialogProps {
  workspaceId: string;
  workspaceName: string;
  isOpen: boolean;
  onClose: () => void;
  onMigrationComplete?: (report: MigrationReport) => void;
}

export function MigrationDialog({
  workspaceId,
  workspaceName,
  isOpen,
  onClose,
  onMigrationComplete,
}: MigrationDialogProps) {
  const { isMigrating, migrationProgress, error, migrateWorkspace } = useMigration();
  const [showDetails, setShowDetails] = useState(false);

  if (!isOpen) return null;

  const handleMigrate = async () => {
    try {
      logger.info('[MigrationDialog] Starting migration for workspace:', workspaceId);
      const report = await migrateWorkspace(workspaceId);
      
      if (report.success) {
        logger.info('[MigrationDialog] Migration successful');
        onMigrationComplete?.(report);
      }
    } catch (err) {
      logger.error('[MigrationDialog] Migration failed:', err);
    }
  };

  const handleSkip = () => {
    logger.info('[MigrationDialog] User skipped migration');
    onClose();
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  const formatDuration = (ms: number): string => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-2xl w-full mx-4 p-6">
        {/* Header */}
        <div className="mb-4">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Workspace Migration Available
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Workspace: <span className="font-medium">{workspaceName}</span>
          </p>
        </div>

        {/* Content */}
        {!migrationProgress ? (
          <div className="space-y-4">
            <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
              <h3 className="font-semibold text-blue-900 dark:text-blue-100 mb-2">
                Why migrate?
              </h3>
              <ul className="text-sm text-blue-800 dark:text-blue-200 space-y-1 list-disc list-inside">
                <li>Improved search performance</li>
                <li>Better handling of nested archives</li>
                <li>Automatic deduplication saves disk space</li>
                <li>More reliable file access</li>
              </ul>
            </div>

            <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
              <h3 className="font-semibold text-yellow-900 dark:text-yellow-100 mb-2">
                What happens during migration?
              </h3>
              <ul className="text-sm text-yellow-800 dark:text-yellow-200 space-y-1 list-disc list-inside">
                <li>Files are converted to content-addressable storage (CAS)</li>
                <li>Duplicate files are automatically detected and deduplicated</li>
                <li>Original files remain accessible during migration</li>
                <li>Migration can take a few minutes depending on workspace size</li>
              </ul>
            </div>

            {error && (
              <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
                <p className="text-sm text-red-800 dark:text-red-200">
                  <span className="font-semibold">Error:</span> {error}
                </p>
              </div>
            )}
          </div>
        ) : (
          <div className="space-y-4">
            {/* Migration Results */}
            <div className={`border rounded-lg p-4 ${
              migrationProgress.success
                ? 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800'
                : 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800'
            }`}>
              <h3 className={`font-semibold mb-3 ${
                migrationProgress.success
                  ? 'text-green-900 dark:text-green-100'
                  : 'text-yellow-900 dark:text-yellow-100'
              }`}>
                {migrationProgress.success ? '✓ Migration Completed' : '⚠ Migration Completed with Warnings'}
              </h3>

              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Total Files</p>
                  <p className="font-semibold text-gray-900 dark:text-white">
                    {migrationProgress.total_files}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Migrated</p>
                  <p className="font-semibold text-green-600 dark:text-green-400">
                    {migrationProgress.migrated_files}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Deduplicated</p>
                  <p className="font-semibold text-blue-600 dark:text-blue-400">
                    {migrationProgress.deduplicated_files}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Failed</p>
                  <p className={`font-semibold ${
                    migrationProgress.failed_files > 0
                      ? 'text-red-600 dark:text-red-400'
                      : 'text-gray-600 dark:text-gray-400'
                  }`}>
                    {migrationProgress.failed_files}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Original Size</p>
                  <p className="font-semibold text-gray-900 dark:text-white">
                    {formatBytes(migrationProgress.original_size)}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">CAS Size</p>
                  <p className="font-semibold text-gray-900 dark:text-white">
                    {formatBytes(migrationProgress.cas_size)}
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Space Saved</p>
                  <p className="font-semibold text-green-600 dark:text-green-400">
                    {formatBytes(migrationProgress.original_size - migrationProgress.cas_size)}
                    {' '}
                    ({((1 - migrationProgress.cas_size / migrationProgress.original_size) * 100).toFixed(1)}%)
                  </p>
                </div>
                <div>
                  <p className="text-gray-600 dark:text-gray-400">Duration</p>
                  <p className="font-semibold text-gray-900 dark:text-white">
                    {formatDuration(migrationProgress.duration_ms)}
                  </p>
                </div>
              </div>

              {migrationProgress.failed_files > 0 && (
                <div className="mt-4">
                  <button
                    onClick={() => setShowDetails(!showDetails)}
                    className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
                  >
                    {showDetails ? 'Hide' : 'Show'} failed files ({migrationProgress.failed_file_paths.length})
                  </button>
                  {showDetails && (
                    <div className="mt-2 max-h-40 overflow-y-auto bg-white dark:bg-gray-900 rounded p-2 text-xs font-mono">
                      {migrationProgress.failed_file_paths.map((path, idx) => (
                        <div key={idx} className="text-red-600 dark:text-red-400">
                          {path}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3 mt-6">
          {!migrationProgress ? (
            <>
              <Button
                variant="secondary"
                onClick={handleSkip}
                disabled={isMigrating}
              >
                Skip for Now
              </Button>
              <Button
                variant="primary"
                onClick={handleMigrate}
                disabled={isMigrating}
              >
                {isMigrating ? 'Migrating...' : 'Migrate Now'}
              </Button>
            </>
          ) : (
            <Button variant="primary" onClick={onClose}>
              Close
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
