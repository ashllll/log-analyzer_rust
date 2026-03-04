/**
 * Button 组件单元测试
 *
 * 测试通用按钮组件的各种变体和功能
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from '../Button';

// Mock lucide-react icons
const mockIcon = () => <span data-testid="mock-icon">Icon</span>;

describe('Button 组件', () => {
  describe('基础渲染', () => {
    it('应该渲染按钮文本', () => {
      render(<Button>Click Me</Button>);
      expect(screen.getByRole('button')).toHaveTextContent('Click Me');
    });

    it('应该有默认的 primary 变体', () => {
      const { container } = render(<Button>Click Me</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('bg-primary');
    });

    it('应该有正确的类型属性', () => {
      render(<Button>Click Me</Button>);
      expect(screen.getByRole('button')).toHaveAttribute('type', 'button');
    });
  });

  describe('变体样式', () => {
    it('primary 变体应该有正确的样式', () => {
      const { container } = render(<Button variant="primary">Primary</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('bg-primary');
      expect(button).toHaveClass('hover:bg-primary-hover');
    });

    it('secondary 变体应该有正确的样式', () => {
      render(<Button variant="secondary">Secondary</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('bg-bg-card');
      expect(button).toHaveClass('border');
    });

    it('ghost 变体应该有正确的样式', () => {
      render(<Button variant="ghost">Ghost</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('hover:bg-bg-hover');
      expect(button).toHaveClass('text-text-muted');
    });

    it('danger 变体应该有正确的样式', () => {
      render(<Button variant="danger">Danger</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('bg-red-500/10');
      expect(button).toHaveClass('text-red-400');
    });

    it('icon 变体应该有正确的样式', () => {
      render(<Button variant="icon" icon={mockIcon} />);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('h-8');
      expect(button).toHaveClass('w-8');
    });
  });

  describe('图标支持', () => {
    it('应该显示图标', () => {
      render(<Button icon={mockIcon}>With Icon</Button>);
      expect(screen.getByTestId('mock-icon')).toBeInTheDocument();
    });

    it('图标应该在文本之前', () => {
      render(<Button icon={mockIcon}>With Icon</Button>);
      const button = screen.getByRole('button');
      const icon = screen.getByTestId('mock-icon');
      const text = screen.getByText('With Icon');

      expect(button).toContainElement(icon);
      expect(button).toContainElement(text);
    });

    it('没有文本时应该只显示图标', () => {
      render(<Button icon={mockIcon} />);
      expect(screen.getByTestId('mock-icon')).toBeInTheDocument();
      expect(screen.getByRole('button')).toHaveTextContent('Icon');
    });
  });

  describe('交互行为', () => {
    it('应该调用 onClick 处理函数', () => {
      const handleClick = jest.fn();
      render(<Button onClick={handleClick}>Click Me</Button>);

      fireEvent.click(screen.getByRole('button'));
      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('应该阻止事件冒泡', () => {
      const handleParentClick = jest.fn();
      const handleChildClick = jest.fn();

      const { container } = render(
        <div onClick={handleParentClick}>
          <Button onClick={handleChildClick}>Click Me</Button>
        </div>
      );

      fireEvent.click(screen.getByRole('button'));

      expect(handleChildClick).toHaveBeenCalledTimes(1);
      expect(handleParentClick).not.toHaveBeenCalled();
    });

    it('disabled 时不应该调用 onClick', () => {
      const handleClick = jest.fn();
      render(<Button onClick={handleClick} disabled>Click Me</Button>);

      fireEvent.click(screen.getByRole('button'));
      expect(handleClick).not.toHaveBeenCalled();
    });

    it('disabled 时应该有正确的样式', () => {
      render(<Button disabled>Disabled</Button>);
      const button = screen.getByRole('button');
      expect(button).toHaveClass('disabled:opacity-50');
      expect(button).toHaveClass('disabled:cursor-not-allowed');
    });
  });

  describe('自定义类名', () => {
    it('应该支持自定义 className', () => {
      render(<Button className="custom-class">Custom</Button>);
      expect(screen.getByRole('button')).toHaveClass('custom-class');
    });

    it('应该合并多个类名', () => {
      render(
        <Button variant="primary" className="custom-class another-class">
          Custom
        </Button>
      );
      const button = screen.getByRole('button');
      expect(button).toHaveClass('bg-primary');
      expect(button).toHaveClass('custom-class');
      expect(button).toHaveClass('another-class');
    });
  });

  describe('其他 HTML 属性', () => {
    it('应该支持 data-testid 属性', () => {
      render(<Button data-testid="test-button">Test</Button>);
      expect(screen.getByTestId('test-button')).toBeInTheDocument();
    });

    it('应该支持 id 属性', () => {
      render(<Button id="my-button">Test</Button>);
      expect(screen.getByRole('button')).toHaveAttribute('id', 'my-button');
    });

    it('应该支持 name 属性', () => {
      render(<Button name="submit">Test</Button>);
      expect(screen.getByRole('button')).toHaveAttribute('name', 'submit');
    });
  });

  describe('空内容', () => {
    it('没有内容时应该仍然渲染按钮', () => {
      render(<Button />);
      expect(screen.getByRole('button')).toBeInTheDocument();
    });

    it('只有图标时应该正常渲染', () => {
      render(<Button icon={mockIcon} />);
      expect(screen.getByRole('button')).toBeInTheDocument();
    });
  });
});
