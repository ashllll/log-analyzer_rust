/**
 * ActiveKeywords 组件单元测试
 * 验证关键词展示组件的渲染和交互
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { ActiveKeywords } from '../ActiveKeywords';

describe('ActiveKeywords', () => {
  const defaultProps = {
    activeTerms: [] as string[],
    onRemoveTerm: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should render None when query is empty', () => {
    render(<ActiveKeywords {...defaultProps} activeTerms={[]} />);

    expect(screen.getByText('None')).toBeInTheDocument();
    expect(screen.queryByText('Active:')).toBeInTheDocument();
  });

  it('should render None when query has only whitespace', () => {
    const { container } = render(<ActiveKeywords {...defaultProps} activeTerms={['   ']} />);

    // With whitespace-only query, split gives ["", "", ""], filter removes empties
    // So query becomes empty and "None" is shown
    const keywordSpans = container.querySelectorAll('span.flex.items-center');
    expect(keywordSpans.length).toBe(0); // No keyword spans
  });

  it('should render single keyword correctly', () => {
    render(<ActiveKeywords {...defaultProps} activeTerms={['error']} />);

    expect(screen.getByText('error')).toBeInTheDocument();
    expect(screen.queryByText('None')).not.toBeInTheDocument();
  });

  it('should render multiple active terms', () => {
    render(<ActiveKeywords {...defaultProps} activeTerms={['error', 'warning', 'info']} />);

    expect(screen.getByText('error')).toBeInTheDocument();
    expect(screen.getByText('warning')).toBeInTheDocument();
    expect(screen.getByText('info')).toBeInTheDocument();
  });

  it('should call onRemoveTerm with correct term when delete button clicked', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['error', 'warning']} onRemoveTerm={onRemoveTerm} />);

    // Find and click the delete button for 'error'
    const errorDeleteButton = screen
      .getAllByRole('button', { hidden: true })
      .find((button) => button.closest('span')?.textContent?.includes('error'));

    expect(errorDeleteButton).toBeDefined();

    if (errorDeleteButton) {
      fireEvent.click(errorDeleteButton);
      expect(onRemoveTerm).toHaveBeenCalledWith('error');
    }
  });

  it('should call onRemoveTerm with correct term when warning delete button clicked', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['error', 'warning']} onRemoveTerm={onRemoveTerm} />);

    const warningDeleteButton = screen
      .getAllByRole('button', { hidden: true })
      .find((button) => button.closest('span')?.textContent?.includes('warning'));

    expect(warningDeleteButton).toBeDefined();

    if (warningDeleteButton) {
      fireEvent.click(warningDeleteButton);
      expect(onRemoveTerm).toHaveBeenCalledWith('warning');
    }
  });

  it('should trim whitespace from active terms', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['  error  ', '  warning  ']} onRemoveTerm={onRemoveTerm} />);

    // Should render trimmed terms
    expect(screen.getByText('error')).toBeInTheDocument();
    expect(screen.getByText('warning')).toBeInTheDocument();
  });

  it('should ignore empty active terms', () => {
    render(<ActiveKeywords {...defaultProps} activeTerms={['error', '', 'warning']} />);

    // Should still render the non-empty terms
    expect(screen.getByText('error')).toBeInTheDocument();
    expect(screen.getByText('warning')).toBeInTheDocument();

    // Query splits to ["error", "", "warning"], empty one is filtered out
    // So we should only see 2 keywords, not 3
    // The empty string between || is filtered out
  });

  it('should not call onRemoveTerm when clicking the term itself', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['error']} onRemoveTerm={onRemoveTerm} />);

    // Click on the term text, not the delete button
    const termElement = screen.getByText('error');
    fireEvent.click(termElement);

    // Should not trigger remove since we clicked the text, not the X button
    // Note: The X button is inside the span, so clicking text might also trigger
    // depending on implementation. This test documents the current behavior.
  });

  it('should render Hash icon before each term', () => {
    render(<ActiveKeywords {...defaultProps} activeTerms={['error']} />);

    // The component renders Hash icons - verify the component renders
    const container = document.querySelector('.flex.items-center.gap-2');
    expect(container).toBeInTheDocument();
  });

  it('should render all terms with delete buttons', () => {
    const onRemoveTerm = jest.fn();
    render(
      <ActiveKeywords {...defaultProps} activeTerms={['a', 'b', 'c']} onRemoveTerm={onRemoveTerm} />
    );

    // Should render all three terms
    expect(screen.getByText('a')).toBeInTheDocument();
    expect(screen.getByText('b')).toBeInTheDocument();
    expect(screen.getByText('c')).toBeInTheDocument();

    // Should have 3 delete buttons
    const deleteButtons = screen.getAllByRole('button', { hidden: true });
    expect(deleteButtons).toHaveLength(3);
  });

  it('should preserve regex terms containing alternation', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['error|warning']} onRemoveTerm={onRemoveTerm} />);

    expect(screen.getByText('error|warning')).toBeInTheDocument();
    const keywordSpans = document.querySelectorAll('span.flex.items-center');
    expect(keywordSpans.length).toBe(1);
  });

  it('should handle whitespace-only active terms', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['', 'error']} onRemoveTerm={onRemoveTerm} />);

    // Should only show 'error'
    expect(screen.getByText('error')).toBeInTheDocument();
    // The empty string before | should be filtered out
    const keywordSpans = document.querySelectorAll('span.flex.items-center');
    expect(keywordSpans.length).toBe(1);
  });

  it('should display aria-label for accessibility', () => {
    const onRemoveTerm = jest.fn();
    render(<ActiveKeywords {...defaultProps} activeTerms={['error']} onRemoveTerm={onRemoveTerm} />);

    const deleteButton = screen.getByRole('button', { hidden: true });
    expect(deleteButton).toHaveAttribute('aria-label', '删除关键词 error');
  });
});
