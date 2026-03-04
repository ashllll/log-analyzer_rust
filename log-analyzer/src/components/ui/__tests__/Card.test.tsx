/**
 * Card 组件单元测试
 *
 * 测试卡片的渲染、样式和子元素
 */

import { render, screen } from '@testing-library/react';

import { Card } from '../Card';

describe('Card Component', () => {
  describe('渲染测试', () => {
    it('应该渲染默认卡片', () => {
      render(<Card>Card content</Card>);
      const card = screen.getByText('Card content');

      expect(card).toBeInTheDocument();
      expect(card.parentElement).toHaveClass('bg-bg-card', 'border', 'rounded-lg');
    });

    it('应该渲染嵌套内容', () => {
      render(
        <Card>
          <h2>Title</h2>
          <p>Content</p>
        </Card>
      );

      expect(screen.getByText('Title')).toBeInTheDocument();
      expect(screen.getByText('Content')).toBeInTheDocument();
    });

    it('应该支持空卡片', () => {
      const { container } = render(<Card />);
      const card = container.firstChild as HTMLElement;

      expect(card).toBeInTheDocument();
      expect(card).toHaveClass('bg-bg-card');
    });

    it('应该支持自定义类名', () => {
      render(<Card className="custom-class">Content</Card>);
      const card = screen.getByText('Card content');

      expect(card.parentElement).toHaveClass('custom-class');
    });
  });

  describe('样式类', () => {
    it('应该有基础样式类', () => {
      render(<Card>Content</Card>);
      const card = screen.getByText('Card content');

      expect(card.parentElement).toHaveClass(
        'bg-bg-card',
        'border',
        'border-border-base',
        'rounded-lg',
        'overflow-hidden'
      );
    });

    it('应该支持多个自定义类名', () => {
      render(
        <Card className="p-4 shadow-lg hover:shadow-xl">
          Content
        </Card>
      );
      const card = screen.getByText('Card content');

      expect(card.parentElement).toHaveClass(
        'p-4',
        'shadow-lg',
        'hover:shadow-xl'
      );
    });
  });

  describe('属性传递', () => {
    it('应该正确传递 HTML 属性', () => {
      render(<Card data-testid="test-card">Content</Card>);
      const card = screen.getByTestId('test-card');

      expect(card).toBeInTheDocument();
    });

    it('应该支持 onClick 事件', () => {
      const handleClick = () => {};
      render(<Card onClick={handleClick}>Clickable</Card>);
      const card = screen.getByText('Clickable');

      expect(card).toBeInTheDocument();
    });

    it('应该支持 style 属性', () => {
      render(<Card style={{ backgroundColor: 'red' }}>Styled</Card>);
      const card = screen.getByText('Styled');

      expect(card.parentElement).toHaveStyle({ backgroundColor: 'red' });
    });
  });

  describe('子元素渲染', () => {
    it('应该渲染文本内容', () => {
      render(<Card>Simple text</Card>);
      expect(screen.getByText('Simple text')).toBeInTheDocument();
    });

    it('应该渲染组件子元素', () => {
      const TestComponent = () => <div>Test Component</div>;
      render(
        <Card>
          <TestComponent />
        </Card>
      );

      expect(screen.getByText('Test Component')).toBeInTheDocument();
    });

    it('应该渲染多个子元素', () => {
      render(
        <Card>
          <div>First</div>
          <div>Second</div>
          <div>Third</div>
        </Card>
      );

      expect(screen.getByText('First')).toBeInTheDocument();
      expect(screen.getByText('Second')).toBeInTheDocument();
      expect(screen.getByText('Third')).toBeInTheDocument();
    });
  });
});
