// Mock logger for testing
export const logger = {
  debug: jest.fn((_message: string, ..._args: any[]) => {
    // Silent in tests
  }),
  
  info: jest.fn((_message: string, ..._args: any[]) => {
    // Silent in tests
  }),
  
  warn: jest.fn((_message: string, ..._args: any[]) => {
    // Silent in tests
  }),
  
  error: jest.fn((_message: string, ..._args: any[]) => {
    // Silent in tests
  })
};
