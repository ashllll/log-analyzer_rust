import React from 'react';
import { render, screen } from '@testing-library/react';
import FilterPalette from '../FilterPalette';
import type { KeywordGroup } from '../../../types/common';

const enabledGroup: KeywordGroup = {
  id: 'g1',
  name: 'Errors',
  color: 'red',
  patterns: [{ regex: 'error', comment: '' }],
  enabled: true,
};

const disabledGroup: KeywordGroup = {
  id: 'g2',
  name: 'Debug',
  color: 'blue',
  patterns: [{ regex: 'debug', comment: '' }],
  enabled: false,
};

describe('FilterPalette', () => {
  it('renders patterns from enabled keyword groups', () => {
    render(
      <FilterPalette
        isOpen={true}
        onClose={jest.fn()}
        groups={[enabledGroup]}
        activeTerms={[]}
        onToggleRule={jest.fn()}
      />,
    );

    expect(screen.getByText('error')).toBeInTheDocument();
  });

  it('does NOT render patterns from disabled keyword groups', () => {
    render(
      <FilterPalette
        isOpen={true}
        onClose={jest.fn()}
        groups={[disabledGroup]}
        activeTerms={[]}
        onToggleRule={jest.fn()}
      />,
    );

    expect(screen.queryByText('debug')).not.toBeInTheDocument();
  });

  it('renders only enabled groups when mixed enabled/disabled groups are passed', () => {
    render(
      <FilterPalette
        isOpen={true}
        onClose={jest.fn()}
        groups={[enabledGroup, disabledGroup]}
        activeTerms={[]}
        onToggleRule={jest.fn()}
      />,
    );

    expect(screen.getByText('error')).toBeInTheDocument();
    expect(screen.queryByText('debug')).not.toBeInTheDocument();
  });

  it('renders nothing when isOpen is false', () => {
    render(
      <FilterPalette
        isOpen={false}
        onClose={jest.fn()}
        groups={[enabledGroup]}
        activeTerms={[]}
        onToggleRule={jest.fn()}
      />,
    );

    expect(screen.queryByText('error')).not.toBeInTheDocument();
  });
});
