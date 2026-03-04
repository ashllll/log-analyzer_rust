/**
 * NavItem 组件单元测试
 *
 * 测试导航项的渲染、激活状态和点击行为
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { NavItem } from '../NavItem';
import { Search } from 'lucide-react';

describe('NavItem Component', () => {
  describe('渲染测试', () => {
    it('应该渲染默认非激活状态的导航项', () => {
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: /search/i });

      expect(button).toBeInTheDocument();
      expect(button).toHaveClass('text-text-muted', 'hover:bg-bg-hover');
      expect(button).not.toHaveClass('bg-primary');
    });

    it('应该渲染激活状态的导航项', () => {
      render(
        <NavItem icon={Search} label="Search" active={true} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: /search/i });

      expect(button).toHaveClass('bg-primary', 'text-white');
    });

    it('应该渲染图标', () => {
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: /search/i });
      const svg = button.querySelector('svg');

      expect(svg).toBeInTheDocument();
    });

    it('应该渲染标签文本', () => {
      render(
        <NavItem icon={Search} label="Workspaces" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button');

      expect(button).toHaveTextContent('Workspaces');
    });
  });

  describe('交互行为', () => {
    it('应该在点击时调用 onClick 处理函数', async () => {
      const handleClick = jest.fn();
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={handleClick} />
      );

      const button = screen.getByRole('button', { name: /search/i });
      await userEvent.click(button);

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('应该传递事件对象到 onClick', async () => {
      const handleClick = jest.fn();
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={handleClick} />
      );

      const button = screen.getByRole('button', { name: /search/i });
      await userEvent.click(button);

      expect(handleClick).toHaveBeenCalled();
      const call = handleClick.mock.calls[0];
      // React Testing Library 的 userEvent.click 传递的是 React 的 SyntheticEvent
      expect(call[0]).toBeDefined();
    });
  });

  describe('属性传递', () => {
    it('应该支持 data-testid 属性', () => {
      render(
        <NavItem
          icon={Search}
          label="Search"
          active={false}
          onClick={() => {}}
          data-testid="nav-search"
        />
      );
      const button = screen.getByTestId('nav-search');

      expect(button).toBeInTheDocument();
    });
  });

  describe('样式类', () => {
    it('应该有基础样式类', () => {
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: /search/i });

      expect(button).toHaveClass(
        'w-full',
        'flex',
        'items-center',
        'gap-3',
        'px-3',
        'py-2',
        'rounded-md',
        'transition-all'
      );
    });

    it('激活状态应该有不同的样式', () => {
      const { rerender } = render(
        <NavItem icon={Search} label="Search" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: /search/i });

      expect(button).not.toHaveClass('bg-primary');

      rerender(
        <NavItem icon={Search} label="Search" active={true} onClick={() => {}} />
      );

      expect(button).toHaveClass('bg-primary', 'text-white');
    });
  });

  describe('无障碍性', () => {
    it('应该有正确的按钮角色', () => {
      render(
        <NavItem icon={Search} label="Search" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button');

      expect(button).toBeInTheDocument();
    });

    it('应该通过标签文本可访问', () => {
      render(
        <NavItem icon={Search} label="Workspaces" active={false} onClick={() => {}} />
      );
      const button = screen.getByRole('button', { name: 'Workspaces' });

      expect(button).toBeInTheDocument();
    });
  });
});
