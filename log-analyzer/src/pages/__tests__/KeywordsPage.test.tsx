import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import KeywordsPage from '../KeywordsPage';

// Mock the useKeywordManager hook
const mockSaveKeywordGroup = jest.fn();
const mockDeleteKeywordGroup = jest.fn();
const mockToggleKeywordGroup = jest.fn();

jest.mock('../../hooks/useKeywordManager', () => ({
  useKeywordManager: () => ({
    keywordGroups: [
      {
        id: 'g1',
        name: 'Critical Errors',
        color: 'red',
        patterns: [{ regex: 'error', comment: 'errors' }],
        enabled: true,
      },
      {
        id: 'g2',
        name: 'Debug Info',
        color: 'blue',
        patterns: [{ regex: 'debug', comment: '' }],
        enabled: false,
      },
    ],
    saveKeywordGroup: mockSaveKeywordGroup,
    deleteKeywordGroup: mockDeleteKeywordGroup,
    toggleKeywordGroup: mockToggleKeywordGroup,
    loading: false,
    error: null,
  }),
}));

// Mock lucide-react icons used by KeywordsPage
jest.mock('lucide-react', () => {
  const icons: Record<string, React.FC<{ size?: number; className?: string }>> = {};
  return new Proxy(icons, {
    get: (_target, prop: string) => {
      if (prop === '__esModule') return true;
      const Icon = (props: { size?: number; className?: string }) =>
        React.createElement('svg', { 'data-testid': `icon-${prop}`, ...props });
      Icon.displayName = prop;
      return Icon;
    },
  });
});

// Mock framer-motion to avoid animation issues in tests
jest.mock('framer-motion', () => {
  const React = require('react');
  return {
    motion: new Proxy({}, {
      get: (_target, prop: string) => {
        // Create a component for each HTML element (div, span, etc.)
        return React.forwardRef(
          (props: React.PropsWithChildren<Record<string, unknown>>, ref: React.Ref<unknown>) => {
            const {
              variants, initial, animate, exit, transition, whileHover, whileTap,
              ...rest
            } = props;
            return React.createElement(prop, { ...rest, ref });
          }
        );
      },
    }),
    AnimatePresence: ({ children }: React.PropsWithChildren) => children,
  };
});

describe('KeywordsPage', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders keyword groups', () => {
    render(<KeywordsPage />);
    expect(screen.getByText('Critical Errors')).toBeInTheDocument();
    expect(screen.getByText('Debug Info')).toBeInTheDocument();
  });

  it('shows an enable/disable toggle for each keyword group', () => {
    render(<KeywordsPage />);

    // Each group should have a toggle button (aria-label contains the group name)
    const toggles = screen.getAllByRole('button', { name: /toggle/i });
    expect(toggles.length).toBeGreaterThanOrEqual(2);
  });

  it('calls toggleKeywordGroup when toggle is clicked', () => {
    render(<KeywordsPage />);

    const toggleButton = screen.getByLabelText(/toggle.*Critical Errors/i);
    fireEvent.click(toggleButton);

    expect(mockToggleKeywordGroup).toHaveBeenCalledWith('g1');
  });

  it('displays visual enabled/disabled state', () => {
    render(<KeywordsPage />);

    const toggleButton1 = screen.getByLabelText(/toggle.*Critical Errors/i);
    const toggleButton2 = screen.getByLabelText(/toggle.*Debug Info/i);

    expect(toggleButton1).toHaveAttribute('aria-pressed', 'true');
    expect(toggleButton2).toHaveAttribute('aria-pressed', 'false');
  });
});
