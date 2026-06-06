import { useCallback, useState } from 'react';
import { useToast } from './useToast';
import { getFullErrorMessage } from '../services/errors';

export interface AsyncActionOptions<T = unknown> {
  /** If set, shows a success toast with this message after the action resolves. */
  successMessage?: string;
  /** If set, shows an error toast prefixed with this text. The full error message is appended. */
  errorPrefix?: string;
  /** If true, re-throws the caught error after setting internal error state / showing toast. Default false. */
  rethrow?: boolean;
  /** Called after the action resolves successfully, before any toast. */
  onSuccess?: (result: T) => void;
  /** Called after the action rejects. Toast (if errorPrefix is set) fires first. */
  onError?: (error: unknown) => void;
}

export interface UseAsyncActionResult {
  /** Wrap an async operation. Loading state, error state, and optional toasts are managed automatically. */
  execute: <T>(action: () => Promise<T>, opts?: AsyncActionOptions<T>) => Promise<T | undefined>;
  /** True while the action is running. */
  isLoading: boolean;
  /** The most recent error message (from getFullErrorMessage). Reset on each execute. */
  error: string | null;
  /** Manually clear the error state. */
  clearError: () => void;
}

/**
 * Composable hook that wraps an async action with:
 * - Loading state management (isLoading)
 * - Error state management (error)
 * - Optional success/error toast via {@link AsyncActionOptions.successMessage} / {@link AsyncActionOptions.errorPrefix}
 *
 * Eliminates the duplicated `useState(false)` + `try/catch/finally` pattern
 * spread across 6+ hooks.
 */
export const useAsyncAction = (): UseAsyncActionResult => {
  const { showToast: addToast } = useToast();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearError = useCallback(() => setError(null), []);

  const execute = useCallback(
    async <T>(action: () => Promise<T>, opts?: AsyncActionOptions<T>): Promise<T | undefined> => {
      setIsLoading(true);
      setError(null);
      try {
        const result = await action();
        if (opts?.successMessage) {
          addToast('success', opts.successMessage);
        }
        opts?.onSuccess?.(result);
        return result;
      } catch (e) {
        const msg = getFullErrorMessage(e);
        setError(msg);
        if (opts?.errorPrefix) {
          addToast('error', `${opts.errorPrefix}: ${msg}`);
        }
        opts?.onError?.(e);
        if (opts?.rethrow) throw e;
        return undefined;
      } finally {
        setIsLoading(false);
      }
    },
    [addToast],
  );

  return { execute, isLoading, error, clearError };
};
