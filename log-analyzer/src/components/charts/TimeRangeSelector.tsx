/**
 * 时间范围选择器组件
 *
 * 用于选择性能指标数据的时间范围
 * 预设选项：1h、6h、24h、7d、30d
 */

import { Clock, Calendar } from 'lucide-react';

// ============================================================================
// 类型定义
// ============================================================================

export type TimeRangeValue = '1h' | '6h' | '24h' | '7d' | '30d';

export interface TimeRangeOption {
  value: TimeRangeValue;
  label: string;
  icon?: React.ReactNode;
  description?: string;
}

export const TIME_RANGE_OPTIONS: TimeRangeOption[] = [
  {
    value: '1h',
    label: '1 Hour',
    icon: <Clock size={14} />,
    description: 'Last hour',
  },
  {
    value: '6h',
    label: '6 Hours',
    icon: <Clock size={14} />,
    description: 'Last 6 hours',
  },
  {
    value: '24h',
    label: '24 Hours',
    icon: <Clock size={14} />,
    description: 'Last 24 hours',
  },
  {
    value: '7d',
    label: '7 Days',
    icon: <Calendar size={14} />,
    description: 'Last 7 days',
  },
  {
    value: '30d',
    label: '30 Days',
    icon: <Calendar size={14} />,
    description: 'Last 30 days',
  },
];

// ============================================================================
// 组件属性
// ============================================================================

interface TimeRangeSelectorProps {
  /** 当前选中的时间范围 */
  value: TimeRangeValue;
  /** 时间范围变化回调 */
  onChange: (value: TimeRangeValue) => void;
  /** 是否禁用 */
  disabled?: boolean;
  /** 尺寸变体 */
  size?: 'sm' | 'md';
}

// ============================================================================
// 主组件
// ============================================================================

export function TimeRangeSelector({
  value,
  onChange,
  disabled = false,
  size = 'md',
}: TimeRangeSelectorProps) {
  const buttonClass = size === 'sm' ? 'px-2 py-1 text-xs' : 'px-3 py-1.5 text-sm';

  return (
    <div className="flex items-center gap-1 bg-bg-subtle border border-border-base rounded-lg p-1">
      {TIME_RANGE_OPTIONS.map((option) => {
        const isActive = value === option.value;

        return (
          <button
            key={option.value}
            onClick={() => onChange(option.value)}
            disabled={disabled}
            className={`
              flex items-center gap-1.5 rounded-md transition-all duration-150
              ${buttonClass}
              ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              ${
                isActive
                  ? 'bg-bg-surface text-text-main shadow-sm'
                  : 'text-text-dim hover:text-text-main hover:bg-bg-surface/50'
              }
            `}
            title={option.description}
          >
            {option.icon && <span className="opacity-70">{option.icon}</span>}
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

export default TimeRangeSelector;
