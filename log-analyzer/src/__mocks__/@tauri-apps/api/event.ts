/**
 * Mock Tauri event API for testing
 */
export const listen = jest.fn(() => Promise.resolve(() => {}));
export const emit = jest.fn(() => Promise.resolve());
