/**
 * Sync Status Indicator Component
 * 
 * Visual indicators for real-time state changes and synchronization status.
 * 
 * Requirements: 2.3 - Visual indicators for real-time state changes
 */

import React, { useEffect, useState } from 'react';
import { ConnectionStatusIndicator, ConnectionDot } from './ui/ConnectionStatus';
import { useStateSynchronization } from '../hooks/useStateSynchronization';

// ============================================================================
// Props
// ============================================================================

export interface SyncStatusIndicatorProps {
  showDetails?: boolean;
  compact?: boolean;
  className?: string;
}

// ============================================================================
// Component
// ============================================================================

export const SyncStatusIndicator: React.FC<SyncStatusIndicatorProps> = ({
  showDetails = false,
  compact = false,
  className = '',
}) => {
  const { syncState, connectionStatus, connect } = useStateSynchronization();
  const [showSyncAnimation, setShowSyncAnimation] = useState(false);

  // Show sync animation when last sync time changes
  useEffect(() => {
    if (syncState.lastSyncTime) {
      setShowSyncAnimation(true);
      const timer = setTimeout(() => setShowSyncAnimation(false), 1000);
      return () => clearTimeout(timer);
    }
  }, [syncState.lastSyncTime]);

  if (compact) {
    return (
      <div className={`flex items-center gap-2 ${className}`}>
        <ConnectionDot status={connectionStatus} size="sm" />
        {showSyncAnimation && (
          <span className="text-xs text-green-500 animate-pulse">●</span>
        )}
        {syncState.pendingUpdates > 0 && (
          <span className="text-xs text-yellow-500">
            {syncState.pendingUpdates}
          </span>
        )}
      </div>
    );
  }

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      <ConnectionStatusIndicator
        status={connectionStatus}
        reconnectAttempts={syncState.isConnected ? 0 : 1}
        onReconnect={connect}
        showDetails={showDetails}
      />

      {showDetails && (
        <div className="flex flex-col gap-1 text-xs text-gray-500">
          {/* Sync status */}
          <div className="flex items-center gap-2">
            <span>Sync:</span>
            {syncState.isSyncing ? (
              <span className="text-yellow-600 animate-pulse">Syncing...</span>
            ) : syncState.isConnected ? (
              <span className="text-green-600">Active</span>
            ) : (
              <span className="text-gray-400">Inactive</span>
            )}
          </div>

          {/* Last sync time */}
          {syncState.lastSyncTime && (
            <div className="flex items-center gap-2">
              <span>Last sync:</span>
              <span>{formatRelativeTime(syncState.lastSyncTime)}</span>
              {showSyncAnimation && (
                <span className="text-green-500 animate-ping">●</span>
              )}
            </div>
          )}

          {/* Pending updates */}
          {syncState.pendingUpdates > 0 && (
            <div className="flex items-center gap-2">
              <span>Pending:</span>
              <span className="text-yellow-600">{syncState.pendingUpdates} updates</span>
            </div>
          )}

          {/* Errors */}
          {syncState.syncErrors.length > 0 && (
            <div className="flex items-center gap-2">
              <span className="text-red-600">
                {syncState.syncErrors.length} error(s)
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// ============================================================================
// Real-time Update Flash Component
// ============================================================================

export interface UpdateFlashProps {
  show: boolean;
  type?: 'success' | 'warning' | 'error';
  children: React.ReactNode;
  className?: string;
}

export const UpdateFlash: React.FC<UpdateFlashProps> = ({
  show,
  type = 'success',
  children,
  className = '',
}) => {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (show) {
      setIsVisible(true);
      const timer = setTimeout(() => setIsVisible(false), 500);
      return () => clearTimeout(timer);
    }
  }, [show]);

  const flashColors = {
    success: 'ring-green-400 bg-green-50',
    warning: 'ring-yellow-400 bg-yellow-50',
    error: 'ring-red-400 bg-red-50',
  };

  return (
    <div
      className={`
        transition-all duration-300
        ${isVisible ? `ring-2 ${flashColors[type]}` : ''}
        ${className}
      `}
    >
      {children}
    </div>
  );
};

// ============================================================================
// Sync Badge Component
// ============================================================================

export interface SyncBadgeProps {
  isSynced: boolean;
  isPending?: boolean;
  className?: string;
}

export const SyncBadge: React.FC<SyncBadgeProps> = ({
  isSynced,
  isPending = false,
  className = '',
}) => {
  if (isPending) {
    return (
      <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800 ${className}`}>
        <span className="w-1.5 h-1.5 mr-1 rounded-full bg-yellow-500 animate-pulse" />
        Syncing
      </span>
    );
  }

  if (isSynced) {
    return (
      <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800 ${className}`}>
        <span className="w-1.5 h-1.5 mr-1 rounded-full bg-green-500" />
        Synced
      </span>
    );
  }

  return (
    <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600 ${className}`}>
      <span className="w-1.5 h-1.5 mr-1 rounded-full bg-gray-400" />
      Offline
    </span>
  );
};

// ============================================================================
// Helper Functions
// ============================================================================

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);

  if (diffSec < 5) return 'just now';
  if (diffSec < 60) return `${diffSec}s ago`;
  
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour}h ago`;
  
  return date.toLocaleDateString();
}

export default SyncStatusIndicator;
