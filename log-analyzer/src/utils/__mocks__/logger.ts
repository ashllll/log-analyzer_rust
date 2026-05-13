// Mock logger for testing
type LogArg = unknown;

export const logger = {
  debug: jest.fn((_message: string, ..._args: LogArg[]) => {
    // Silent in tests
  }),
  
  info: jest.fn((_message: string, ..._args: LogArg[]) => {
    // Silent in tests
  }),
  
  warn: jest.fn((_message: string, ..._args: LogArg[]) => {
    // Silent in tests
  }),
  
  error: jest.fn((_message: string, ..._args: LogArg[]) => {
    // Silent in tests
  })
};
