/**
 * Connection Status Indicator Component
 * 
 * Visual indicator for WebSocket connection status with error notifications.
 * 
 * Requirements: 2.3 - Connection status indicators and error notifications
 */

import React from 'react';
import { ConnectionStatus as ConnectionStatusType } from '../../types/websocket';

// ============================================================================
// Props
// ============================================================================

export interface ConnectionStatusProps {
  status: ConnectionStatusType;
  reconnectAttempts?: number;
  latency?: number | null;
  onReconnect?: () => void;
  showDetails?: boolean;
  className?: string;
}

// ============================================================================
// Status Configuration
// ============================================================================

const STATUS_CONFIG: Record<ConnectionStatusType, {
  color: string;
  bgColor: string;
  label: string;
  icon: string;
}> = {
  connected: {
    color: 'text-green-600',
    bgColor: 'bg-green-100',
    label: 'Connected',
    icon: '●',
  },
  connecting: {
    color: 'text-yellow-600',
    bgColor: 'bg-yellow-100',
    label: 'Connecting...',
    icon: '◐',
  },
  disconnected: {
    color: 'text-gray-500',
    bgColor: 'bg-gray-100',
    label: 'Disconnected',
    icon: '○',
  },
  reconnecting: {
    color: 'text-orange-600',
    bgColor: 'bg-orange-100',
    label: 'Reconnecting...',
    icon: '◑',
  },
  error: {
    color: 'text-red-600',
    bgColor: 'bg-red-100',
    label: 'Error',
    icon: '✕',
  },
};

// ============================================================================
// Component
// ============================================================================

export const ConnectionStatusIndicator: React.FC<ConnectionStatusProps> = ({
  status,
  reconnectAttempts = 0,
  latency = null,
  onReconnect,
  showDetails = false,
  className = '',
}) => {
  const config = STATUS_CONFIG[status];

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      {/* Status indicator dot */}
      <div className={`flex items-center gap-1.5 px-2 py-1 rounded-full ${config.bgColor}`}>
        <span className={`${config.color} text-sm ${status === 'connecting' || status === 'reconnecting' ? 'animate-pulse' : ''}`}>
          {config.icon}
        </span>
        <span className={`text-xs font-medium ${config.color}`}>
          {config.label}
        </span>
      </div>

      {/* Details */}
      {showDetails && (
        <div className="flex items-center gap-2 text-xs text-gray-500">
          {/* Latency */}
          {latency !== null && status === 'connected' && (
            <span className="px-1.5 py-0.5 bg-gray-100 rounded">
              {latency}ms
            </span>
          )}

          {/* Reconnect attempts */}
          {status === 'reconnecting' && reconnectAttempts > 0 && (
            <span className="text-orange-600">
              Attempt {reconnectAttempts}
            </span>
          )}

          {/* Reconnect button */}
          {(status === 'disconnected' || status === 'error') && onReconnect && (
            <button
              onClick={onReconnect}
              className="px-2 py-0.5 text-xs text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded transition-colors"
            >
              Reconnect
            </button>
          )}
        </div>
      )}
    </div>
  );
};

// ============================================================================
// Compact Version
// ============================================================================

export interface ConnectionDotProps {
  status: ConnectionStatusType;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
  title?: string;
}

export const ConnectionDot: React.FC<ConnectionDotProps> = ({
  status,
  size = 'sm',
  className = '',
  title,
}) => {
  const config = STATUS_CONFIG[status];
  
  const sizeClasses = {
    sm: 'w-2 h-2',
    md: 'w-3 h-3',
    lg: 'w-4 h-4',
  };

  const colorClasses: Record<ConnectionStatusType, string> = {
    connected: 'bg-green-500',
    connecting: 'bg-yellow-500 animate-pulse',
    disconnected: 'bg-gray-400',
    reconnecting: 'bg-orange-500 animate-pulse',
    error: 'bg-red-500',
  };

  return (
    <div
      className={`rounded-full ${sizeClasses[size]} ${colorClasses[status]} ${className}`}
      title={title || config.label}
    />
  );
};

// ============================================================================
// Toast Notification for Connection Events
// ============================================================================

export interface ConnectionToastProps {
  status: ConnectionStatusType;
  error?: string | null;
  onDismiss?: () => void;
}

export const ConnectionToast: React.FC<ConnectionToastProps> = ({
  status,
  error,
  onDismiss,
}) => {
  const config = STATUS_CONFIG[status];

  // Only show toast for certain statuses
  if (status === 'connected' || status === 'connecting') {
    return null;
  }

  return (
    <div className={`flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg ${config.bgColor}`}>
      <span className={`text-lg ${config.color}`}>{config.icon}</span>
      <div className="flex-1">
        <p className={`font-medium ${config.color}`}>{config.label}</p>
        {error && (
          <p className="text-sm text-gray-600 mt-0.5">{error}</p>
        )}
      </div>
      {onDismiss && (
        <button
          onClick={onDismiss}
          className="text-gray-400 hover:text-gray-600 transition-colors"
        >
          ✕
        </button>
      )}
    </div>
  );
};

export default ConnectionStatusIndicator;
