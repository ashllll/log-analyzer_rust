import { createApiError, getFullErrorMessage } from '../errors';

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
});
