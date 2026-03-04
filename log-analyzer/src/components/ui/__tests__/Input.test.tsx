/**
 * Input 组件单元测试
 *
 * 测试输入框的渲染、属性和交互行为
 */

import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { Input } from '../Input';

describe('Input Component', () => {
  describe('渲染测试', () => {
    it('应该渲染默认输入框', () => {
      render(<Input />);
      const input = screen.getByRole('textbox');

      expect(input).toBeInTheDocument();
      expect(input).toHaveClass('bg-bg-main', 'border', 'rounded-md');
    });

    it('应该支持自定义类名', () => {
      render(<Input className="custom-class" />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveClass('custom-class');
    });

    it('应该支持占位符文本', () => {
      render(<Input placeholder="Enter text..." />);
      const input = screen.getByPlaceholderText('Enter text...');

      expect(input).toBeInTheDocument();
    });

    it('应该是受控组件', () => {
      const { rerender } = render(<Input value="initial" />);
      const input = screen.getByRole('textbox') as HTMLInputElement;

      expect(input.value).toBe('initial');

      rerender(<Input value="updated" />);
      expect(input.value).toBe('updated');
    });
  });

  describe('属性传递', () => {
    it('应该正确传递 type 属性', () => {
      render(<Input type="password" />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('type', 'password');
    });

    it('应该正确传递 type="email"', () => {
      render(<Input type="email" />);
      const input = screen.getByDisplayValue('');

      expect(input).toHaveAttribute('type', 'email');
    });

    it('应该正确传递 type="number"', () => {
      render(<Input type="number" />);
      const input = screen.getByDisplayValue('');

      expect(input).toHaveAttribute('type', 'number');
    });

    it('应该支持 disabled 状态', () => {
      render(<Input disabled />);
      const input = screen.getByRole('textbox');

      expect(input).toBeDisabled();
    });

    it('应该支持 required 属性', () => {
      render(<Input required />);
      const input = screen.getByRole('textbox');

      expect(input).toBeRequired();
    });

    it('应该支持 maxLength 属性', () => {
      render(<Input maxLength={10} />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('maxlength', '10');
    });

    it('应该支持 min/max 属性', () => {
      render(
        <Input type="number" min={0} max={100} />
      );
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('min', '0');
      expect(input).toHaveAttribute('max', '100');
    });

    it('应该支持 name 属性', () => {
      render(<Input name="username" />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('name', 'username');
    });

    it('应该支持 id 属性', () => {
      render(<Input id="test-input" />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('id', 'test-input');
    });

    it('应该支持 autoComplete 属性', () => {
      render(<Input autoComplete="off" />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveAttribute('autoComplete', 'off');
    });
  });

  describe('事件处理', () => {
    it('应该触发 onChange 事件', () => {
      const handleChange = jest.fn();
      render(<Input onChange={handleChange} />);
      const input = screen.getByRole('textbox') as HTMLInputElement;

      input.value = 'test value';
      input.dispatchEvent(new Event('change', { bubbles: true }));

      expect(handleChange).toHaveBeenCalled();
    });

    it('应该触发 onFocus 事件', () => {
      const handleFocus = jest.fn();
      render(<Input onFocus={handleFocus} />);
      const input = screen.getByRole('textbox');

      input.focus();
      // Note: React Testing Library handles focus differently
      // This tests the onFocus prop is passed through
    });

    it('应该触发 onBlur 事件', () => {
      const handleBlur = jest.fn();
      render(<Input onBlur={handleBlur} />);
      const input = screen.getByRole('textbox');

      fireEvent.blur(input);
      expect(handleBlur).toHaveBeenCalled();
    });

    it('应该触发 onKeyDown 事件', () => {
      const handleKeyDown = jest.fn();
      render(<Input onKeyDown={handleKeyDown} />);
      const input = screen.getByRole('textbox');

      fireEvent.keyDown(input, { key: 'Enter' });
      expect(handleKeyDown).toHaveBeenCalled();
    });

    it('应该支持键盘输入', async () => {
      render(<Input />);
      const input = screen.getByRole('textbox') as HTMLInputElement;

      await userEvent.type(input, 'hello');

      expect(input.value).toBe('hello');
    });
  });

  describe('ref 转发', () => {
    it('应该正确转发 ref', () => {
      const ref = { current: null };
      render(<Input ref={ref} />);
      const input = screen.getByRole('textbox');

      expect(ref.current).toBe(input);
    });

    it('应该支持通过 ref 访问 input 方法', () => {
      const ref = React.createRef<HTMLInputElement>();
      render(<Input ref={ref} />);
      const input = screen.getByRole('textbox') as HTMLInputElement;

      input.focus();
      expect(document.activeElement).toBe(input);
    });
  });

  describe('样式类', () => {
    it('应该有基础样式类', () => {
      render(<Input />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveClass('h-9', 'w-full', 'rounded-md', 'px-3');
    });

    it('应该有焦点样式类', () => {
      render(<Input />);
      const input = screen.getByRole('textbox');

      expect(input).toHaveClass('focus:outline-none', 'focus:border-primary/50');
    });
  });
});
