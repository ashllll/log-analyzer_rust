import { createApiError, getFullErrorMessage, ErrorCode, ApiError, ErrorCategory } from '../errors';

describe('errors', () => {
  it('should preserve raw backend messages for unknown errors', () => {
    const error = createApiError(
      'import_folder',
      new Error('Archive error: Legacy extraction failed: unsupported RAR variant')
    );

    expect(getFullErrorMessage(error)).toBe(
      'Archive error: Legacy extraction failed: unsupported RAR variant'
    );
  });

  describe('ERROR_CODE_CATEGORIES', () => {
    const makeApiError = (code: ErrorCode) =>
      new ApiError('test_command', 'test message', JSON.stringify({ code }));

    it.each([
      // USER errors
      [ErrorCode.VALIDATION_ERROR, ErrorCategory.USER],
      [ErrorCode.PATTERN_ERROR, ErrorCategory.USER],
      [ErrorCode.INVALID_PATH, ErrorCategory.USER],
      [ErrorCode.SECURITY_ERROR, ErrorCategory.USER],
      [ErrorCode.NOT_FOUND, ErrorCategory.USER],
      [ErrorCode.QUERY_EXECUTION_ERROR, ErrorCategory.USER],
      [ErrorCode.PARSE_ERROR, ErrorCategory.USER],
      // FILESYSTEM errors
      [ErrorCode.IO_ERROR, ErrorCategory.FILESYSTEM],
      [ErrorCode.ENCODING_ERROR, ErrorCategory.FILESYSTEM],
      [ErrorCode.FILE_WATCHER_ERROR, ErrorCategory.FILESYSTEM],
      // NETWORK errors
      [ErrorCode.NETWORK_ERROR, ErrorCategory.NETWORK],
      // SYSTEM errors
      [ErrorCode.SEARCH_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.ARCHIVE_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.INDEX_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.DATABASE_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.CONFIG_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.CONCURRENCY_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.RESOURCE_CLEANUP_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.INTERNAL_ERROR, ErrorCategory.SYSTEM],
      [ErrorCode.TIMEOUT_ERROR, ErrorCategory.SYSTEM],
      // UNKNOWN
      [ErrorCode.UNKNOWN, ErrorCategory.UNKNOWN],
    ])('should classify %s as %s', (code, expectedCategory) => {
      const error = makeApiError(code);
      expect(error.getCategory()).toBe(expectedCategory);
    });

    it('should classify unknown structued codes as UNKNOWN', () => {
      const error = new ApiError('test', 'msg', JSON.stringify({ code: 'NONEXISTENT' }));
      expect(error.getCategory()).toBe(ErrorCategory.UNKNOWN);
    });

    it.each([
      [ErrorCode.VALIDATION_ERROR, true],
      [ErrorCode.NOT_FOUND, true],
      [ErrorCode.SEARCH_ERROR, false],
    ])('isUserError() for %s should be %s', (code, expected) => {
      expect(makeApiError(code).isUserError()).toBe(expected);
    });

    it.each([
      [ErrorCode.SEARCH_ERROR, true],
      [ErrorCode.INTERNAL_ERROR, true],
      [ErrorCode.VALIDATION_ERROR, false],
    ])('isSystemError() for %s should be %s', (code, expected) => {
      expect(makeApiError(code).isSystemError()).toBe(expected);
    });
  });
});
