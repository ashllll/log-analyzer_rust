// Setup file for Jest tests
// This file is executed before running tests

import '@testing-library/jest-dom';
import React from 'react';
import { TextEncoder, TextDecoder } from 'util';

// Mock import.meta for Vite environment variables
Object.defineProperty(globalThis, 'import', {
  value: {
    meta: {
      env: {
        DEV: false,
        PROD: true,
        MODE: 'production',
      },
    },
  },
  writable: true,
});

// Polyfill TextEncoder/TextDecoder for react-router-dom
// This is needed because Node.js test environment doesn't have these globals
if (typeof global.TextEncoder === 'undefined') {
  global.TextEncoder = TextEncoder as any;
}
if (typeof global.TextDecoder === 'undefined') {
  global.TextDecoder = TextDecoder as any;
}

// Initialize i18n for tests - must be before any components that use translation
import './i18n';

// Make React available globally for tests
(global as any).React = React;

// Mock react-error-boundary
jest.mock('react-error-boundary', () => ({
  ErrorBoundary: ({ children }: { children: React.ReactNode }) => children,
  useErrorHandler: () => jest.fn(),
}));

// Mock Tauri API for testing
const mockTauri = {
  invoke: jest.fn(),
  event: {
    listen: jest.fn(),
    emit: jest.fn(),
  },
  dialog: {
    open: jest.fn(),
    save: jest.fn(),
  },
  fs: {
    readTextFile: jest.fn(),
    writeTextFile: jest.fn(),
  },
};

// Set up global mocks
if (typeof global !== 'undefined') {
  (global as any).__TAURI__ = mockTauri;
}

if (typeof window !== 'undefined') {
  (window as any).__TAURI_IPC__ = jest.fn();
}

// Suppress console errors in tests
const originalError = console.error;
const originalWarn = console.warn;

beforeAll(() => {
  console.error = jest.fn();
  console.warn = jest.fn();

  // Mock ResizeObserver
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };

  // Mock window.matchMedia (for react-hot-toast)
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: jest.fn().mockImplementation(query => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: jest.fn(),
      removeListener: jest.fn(),
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      dispatchEvent: jest.fn(),
    })),
  });
});

afterAll(() => {
  console.error = originalError;
  console.warn = originalWarn;
});

export {};