// Setup file for Jest tests
// This file is executed before running tests

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
});

afterAll(() => {
  console.error = originalError;
  console.warn = originalWarn;
});

export {};