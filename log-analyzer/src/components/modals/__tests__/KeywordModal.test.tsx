import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import KeywordModal from '../KeywordModal';

describe('KeywordModal', () => {
  it('preserves the existing enabled flag when editing a group', () => {
    const onSave = jest.fn();
    const onClose = jest.fn();

    render(
      <KeywordModal
        isOpen={true}
        onClose={onClose}
        onSave={onSave}
        initialData={{
          id: 'group-1',
          name: 'Critical',
          color: 'red',
          patterns: [{ regex: 'error.*timeout', comment: 'critical path' }],
          enabled: false,
        }}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: 'Save Configuration' }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'group-1',
        enabled: false,
      })
    );
  });
});
