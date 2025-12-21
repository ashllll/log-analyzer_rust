/**
 * Tests for Error Boundary components
 * Tests Property 28: Frontend Error Messages
 * Validates: Requirements 7.2
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import { ErrorBoundary } from 'react-error-boundary';
import { ErrorFallback } from '../ErrorFallback';
import { ErrorBoundaryWrapper } from '../ErrorBoundaryWrapper';

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    error: jest.fn(),
    warn: jest.fn(),
    info: jest.fn(),
    debug: jest.fn(),
  },
}));

// Component that throws an error for testing
const ThrowError: React.FC<{ shouldThrow?: boolean }> = ({ shouldThrow = true }) => {
  if (shouldThrow) {
    throw new Error('Test error for error boundary');
  }
  return <div>No error</div>;
};

describe('Error Boundary Components', () => {
  beforeEach(() => {
    // Suppress console.error for these tests since we're intentionally throwing errors
    jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('ErrorFallback', () => {
    it('should display error message and reset button', () => {
      const mockReset = jest.fn();
      const error = new Error('Test error message');

      render(
        <ErrorFallback 
          error={error} 
          resetErrorBoundary={mockReset}
        />
      );

      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
      expect(screen.getByText(/test error message/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();
    });

    it('should call reset function when try again button is clicked', () => {
      const mockReset = jest.fn();
      const error = new Error('Test error');

      render(
        <ErrorFallback 
          error={error} 
          resetErrorBoundary={mockReset}
        />
      );

      const resetButton = screen.getByRole('button', { name: /try again/i });
      resetButton.click();

      expect(mockReset).toHaveBeenCalledTimes(1);
    });

    it('should display generic message for errors without message', () => {
      const mockReset = jest.fn();
      const error = new Error();

      render(
        <ErrorFallback 
          error={error} 
          resetErrorBoundary={mockReset}
        />
      );

      expect(screen.getByText(/an unexpected error occurred/i)).toBeInTheDocument();
    });
  });

  describe('ErrorBoundaryWrapper', () => {
    it('should render children when no error occurs', () => {
      render(
        <ErrorBoundaryWrapper>
          <ThrowError shouldThrow={false} />
        </ErrorBoundaryWrapper>
      );

      expect(screen.getByText('No error')).toBeInTheDocument();
    });

    it('should catch errors and display fallback UI', () => {
      render(
        <ErrorBoundaryWrapper>
          <ThrowError shouldThrow={true} />
        </ErrorBoundaryWrapper>
      );

      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
      expect(screen.getByText(/test error for error boundary/i)).toBeInTheDocument();
    });

    it('should reset error boundary when reset is triggered', () => {
      const { rerender } = render(
        <ErrorBoundaryWrapper>
          <ThrowError shouldThrow={true} />
        </ErrorBoundaryWrapper>
      );

      // Error should be displayed
      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();

      // Click reset button
      const resetButton = screen.getByRole('button', { name: /try again/i });
      resetButton.click();

      // Re-render with non-throwing component
      rerender(
        <ErrorBoundaryWrapper>
          <ThrowError shouldThrow={false} />
        </ErrorBoundaryWrapper>
      );

      expect(screen.getByText('No error')).toBeInTheDocument();
    });
  });

  describe('Property 28: Frontend Error Messages', () => {
    it('should provide meaningful error messages to users for any frontend operation failure', () => {
      const testCases = [
        { error: new Error('Network request failed'), expectedText: /network request failed/i },
        { error: new Error('Validation error: Invalid input'), expectedText: /validation error.*invalid input/i },
        { error: new Error('Permission denied'), expectedText: /permission denied/i },
        { error: new Error('File not found'), expectedText: /file not found/i },
      ];

      testCases.forEach(({ error, expectedText }) => {
        const mockReset = jest.fn();
        const { unmount } = render(
          <ErrorFallback 
            error={error} 
            resetErrorBoundary={mockReset}
          />
        );

        expect(screen.getByText(expectedText)).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();

        unmount();
      });
    });

    it('should handle different error types gracefully', () => {
      const testCases = [
        { error: new TypeError('Type error'), type: 'TypeError' },
        { error: new ReferenceError('Reference error'), type: 'ReferenceError' },
        { error: new SyntaxError('Syntax error'), type: 'SyntaxError' },
        { error: new Error('Generic error'), type: 'Error' },
      ];

      testCases.forEach(({ error, type }) => {
        const mockReset = jest.fn();
        const { unmount } = render(
          <ErrorFallback 
            error={error} 
            resetErrorBoundary={mockReset}
          />
        );

        // Should display error message regardless of type
        expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();

        unmount();
      });
    });

    it('should provide recovery options for users', () => {
      const mockReset = jest.fn();
      const error = new Error('Test error');

      render(
        <ErrorFallback 
          error={error} 
          resetErrorBoundary={mockReset}
        />
      );

      // Should provide a way to recover
      const resetButton = screen.getByRole('button', { name: /try again/i });
      expect(resetButton).toBeInTheDocument();
      expect(resetButton).toBeEnabled();

      // Should provide helpful information
      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
      expect(screen.getByText(/test error/i)).toBeInTheDocument();
    });
  });

  describe('Integration with React Error Boundary', () => {
    it('should integrate properly with react-error-boundary', () => {
      const onError = jest.fn();

      render(
        <ErrorBoundary
          FallbackComponent={ErrorFallback}
          onError={onError}
        >
          <ThrowError shouldThrow={true} />
        </ErrorBoundary>
      );

      expect(onError).toHaveBeenCalledWith(
        expect.any(Error),
        expect.objectContaining({
          componentStack: expect.any(String)
        })
      );

      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
    });

    it('should reset error boundary and re-render children', () => {
      let shouldThrow = true;
      const TestComponent = () => <ThrowError shouldThrow={shouldThrow} />;

      const { rerender } = render(
        <ErrorBoundary
          FallbackComponent={ErrorFallback}
          onReset={() => { shouldThrow = false; }}
        >
          <TestComponent />
        </ErrorBoundary>
      );

      // Error should be displayed
      expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();

      // Click reset
      const resetButton = screen.getByRole('button', { name: /try again/i });
      resetButton.click();

      // Re-render
      rerender(
        <ErrorBoundary
          FallbackComponent={ErrorFallback}
          onReset={() => { shouldThrow = false; }}
        >
          <TestComponent />
        </ErrorBoundary>
      );

      expect(screen.getByText('No error')).toBeInTheDocument();
    });
  });
});