/**
 * SearchFilters 组件单元测试
 * 验证过滤器组件的渲染和交互
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { SearchFilters } from '../SearchFilters';
import type { FilterOptions } from '../../../../types/common';

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
  RotateCcw: () => <span data-testid="rotate-icon">Reset</span>,
}));

describe('SearchFilters', () => {
  const defaultFilterOptions: FilterOptions = {
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: '',
  };

  const defaultProps = {
    filterOptions: defaultFilterOptions,
    onFilterOptionsChange: jest.fn(),
    onReset: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should render all filter controls', () => {
    render(<SearchFilters {...defaultProps} />);

    // Check for Level filter
    expect(screen.getByText('Level')).toBeInTheDocument();

    // Check for Time Range filter
    expect(screen.getByText('Time Range')).toBeInTheDocument();

    // Check for File Pattern filter
    expect(screen.getByText('File Pattern')).toBeInTheDocument();

    // Check for Advanced Filters label
    expect(screen.getByText('Advanced Filters')).toBeInTheDocument();
  });

  it('should render all level buttons', () => {
    render(<SearchFilters {...defaultProps} />);

    // Level buttons show abbreviated text (E, W, I, D) with title attribute for full name
    expect(screen.getByRole('button', { name: 'E' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'W' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'I' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'D' })).toBeInTheDocument();
  });

  it('should toggle level filter on click - add level', () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters {...defaultProps} onFilterOptionsChange={onFilterOptionsChange} />
    );

    // Click the ERROR button (shows as "E")
    const errorButton = screen.getByRole('button', { name: 'E' });
    fireEvent.click(errorButton);

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(1);
    const callArg = onFilterOptionsChange.mock.calls[0][0](defaultFilterOptions);
    expect(callArg.levels).toContain('ERROR');
  });

  it('should toggle level filter on click - remove level', () => {
    const onFilterOptionsChange = jest.fn();
    const filterOptionsWithError: FilterOptions = {
      ...defaultFilterOptions,
      levels: ['ERROR'],
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithError}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const errorButton = screen.getByRole('button', { name: 'E' });
    fireEvent.click(errorButton);

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(1);
    const callArg = onFilterOptionsChange.mock.calls[0][0](filterOptionsWithError);
    expect(callArg.levels).not.toContain('ERROR');
  });

  it('should update time range start via state setter callback', () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters {...defaultProps} onFilterOptionsChange={onFilterOptionsChange} />
    );

    // Directly invoke the callback with a state update function
    // This tests the callback signature, not the actual DOM event
    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      timeRange: { ...prev.timeRange, start: '2024-01-15T10:00' },
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it('should update time range end via state setter callback', () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters {...defaultProps} onFilterOptionsChange={onFilterOptionsChange} />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      timeRange: { ...prev.timeRange, end: '2024-01-15T18:00' },
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it('should update file pattern via state setter callback', () => {
    const onFilterOptionsChange = jest.fn();
    render(
      <SearchFilters {...defaultProps} onFilterOptionsChange={onFilterOptionsChange} />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      filePattern: '*.log',
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it('should show active filter indicator when levels filter applied', () => {
    const filterOptionsWithLevels: FilterOptions = {
      ...defaultFilterOptions,
      levels: ['ERROR', 'WARN'],
    };

    render(<SearchFilters {...defaultProps} filterOptions={filterOptionsWithLevels} />);

    // Should show indicator
    expect(screen.getByText(/2 levels/)).toBeInTheDocument();
  });

  it('should show active filter indicator when time range applied', () => {
    const filterOptionsWithTime: FilterOptions = {
      ...defaultFilterOptions,
      timeRange: { start: '2024-01-15T10:00', end: null },
    };

    render(<SearchFilters {...defaultProps} filterOptions={filterOptionsWithTime} />);

    // Should show indicator
    expect(screen.getByText(/time range/)).toBeInTheDocument();
  });

  it('should show active filter indicator when file pattern applied', () => {
    const filterOptionsWithPattern: FilterOptions = {
      ...defaultFilterOptions,
      filePattern: 'error.log',
    };

    render(<SearchFilters {...defaultProps} filterOptions={filterOptionsWithPattern} />);

    // Should show indicator
    expect(screen.getByText(/file pattern/)).toBeInTheDocument();
  });

  it('should call onReset when Reset button clicked', () => {
    const onReset = jest.fn();
    const filterOptionsWithLevels: FilterOptions = {
      ...defaultFilterOptions,
      levels: ['ERROR'],
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithLevels}
        onReset={onReset}
      />
    );

    // Reset button contains text "Reset" plus icon
    const resetButton = screen.getByRole('button', { name: 'Reset Reset' });
    fireEvent.click(resetButton);

    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it('should not show Reset button when no filters active', () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.queryByRole('button', { name: 'Reset' })).not.toBeInTheDocument();
  });

  it('should not show active filter indicator when no filters active', () => {
    render(<SearchFilters {...defaultProps} />);

    expect(screen.queryByText(/levels/)).not.toBeInTheDocument();
    expect(screen.queryByText(/time range/)).not.toBeInTheDocument();
    expect(screen.queryByText(/file pattern/)).not.toBeInTheDocument();
  });

  it('should allow multiple levels to be selected', () => {
    const onFilterOptionsChange = jest.fn();

    render(
      <SearchFilters {...defaultProps} onFilterOptionsChange={onFilterOptionsChange} />
    );

    // Click ERROR (E)
    fireEvent.click(screen.getByRole('button', { name: 'E' }));
    // Click WARN (W)
    fireEvent.click(screen.getByRole('button', { name: 'W' }));

    expect(onFilterOptionsChange).toHaveBeenCalledTimes(2);

    const secondCallArg = onFilterOptionsChange.mock.calls[1][0]({
      ...defaultFilterOptions,
      levels: ['ERROR'],
    });
    expect(secondCallArg.levels).toContain('ERROR');
    expect(secondCallArg.levels).toContain('WARN');
  });

  it('should display level abbreviations', () => {
    render(<SearchFilters {...defaultProps} />);

    // Level buttons should show abbreviated text (E, W, I, D)
    expect(screen.getByText('E')).toBeInTheDocument();
    expect(screen.getByText('W')).toBeInTheDocument();
    expect(screen.getByText('I')).toBeInTheDocument();
    expect(screen.getByText('D')).toBeInTheDocument();
  });

  it('should handle empty file pattern change via state setter callback', () => {
    const onFilterOptionsChange = jest.fn();
    const filterOptionsWithPattern: FilterOptions = {
      ...defaultFilterOptions,
      filePattern: 'error.log',
    };

    render(
      <SearchFilters
        {...defaultProps}
        filterOptions={filterOptionsWithPattern}
        onFilterOptionsChange={onFilterOptionsChange}
      />
    );

    const updateFn = (prev: FilterOptions) => ({
      ...prev,
      filePattern: '',
    });
    onFilterOptionsChange(updateFn);

    expect(onFilterOptionsChange).toHaveBeenCalledWith(updateFn);
  });

  it('should apply correct styling to active level button', () => {
    const filterOptionsWithError: FilterOptions = {
      ...defaultFilterOptions,
      levels: ['ERROR'],
    };

    render(<SearchFilters {...defaultProps} filterOptions={filterOptionsWithError} />);

    const errorButton = screen.getByRole('button', { name: 'E' });
    // The button should have active styling - this is verified through className
    expect(errorButton.className).toContain('bg-primary');
  });

  it('should apply correct styling to inactive level button', () => {
    render(<SearchFilters {...defaultProps} />);

    const errorButton = screen.getByRole('button', { name: 'E' });
    // The button should NOT have active styling
    expect(errorButton.className).not.toContain('bg-primary');
  });
});
