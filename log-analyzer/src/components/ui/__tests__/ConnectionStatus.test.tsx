/**
 * ConnectionStatus 组件单元测试
 *
 * 测试连接状态指示器的各种状态和交互
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {
  ConnectionStatusIndicator,
  ConnectionDot,
  ConnectionToast,
} from '../ConnectionStatus';
import type { ConnectionStatus } from '../../../types/websocket';

describe('ConnectionStatusIndicator', () => {
  describe('渲染测试', () => {
    it('应该渲染 connected 状态', () => {
      render(<ConnectionStatusIndicator status={'connected' as ConnectionStatus} />);
      const indicator = screen.getByText('Connected');

      expect(indicator).toBeInTheDocument();
      expect(indicator).toHaveClass('text-green-600');
    });

    it('应该渲染 connecting 状态', () => {
      render(<ConnectionStatusIndicator status={'connecting' as ConnectionStatus} />);
      const indicator = screen.getByText('Connecting...');

      expect(indicator).toBeInTheDocument();
      expect(indicator).toHaveClass('text-yellow-600');
    });

    it('应该渲染 disconnected 状态', () => {
      render(<ConnectionStatusIndicator status={'disconnected' as ConnectionStatus} />);
      const indicator = screen.getByText('Disconnected');

      expect(indicator).toBeInTheDocument();
      expect(indicator).toHaveClass('text-gray-500');
    });

    it('应该渲染 reconnecting 状态', () => {
      render(<ConnectionStatusIndicator status={'reconnecting' as ConnectionStatus} />);
      const indicator = screen.getByText('Reconnecting...');

      expect(indicator).toBeInTheDocument();
      expect(indicator).toHaveClass('text-orange-600');
    });

    it('应该渲染 error 状态', () => {
      render(<ConnectionStatusIndicator status={'error' as ConnectionStatus} />);
      const indicator = screen.getByText('Error');

      expect(indicator).toBeInTheDocument();
      expect(indicator).toHaveClass('text-red-600');
    });
  });

  describe('详细信息显示', () => {
    it('应该显示延迟信息', () => {
      render(
        <ConnectionStatusIndicator
          status={'connected' as ConnectionStatus}
          showDetails={true}
          latency={45}
        />
      );

      expect(screen.getByText('45ms')).toBeInTheDocument();
    });

    it('应该显示重连尝试次数', () => {
      render(
        <ConnectionStatusIndicator
          status={'reconnecting' as ConnectionStatus}
          showDetails={true}
          reconnectAttempts={3}
        />
      );

      expect(screen.getByText('Attempt 3')).toBeInTheDocument();
    });

    it('应该显示重连按钮（disconnected 状态）', () => {
      const handleReconnect = jest.fn();
      render(
        <ConnectionStatusIndicator
          status={'disconnected' as ConnectionStatus}
          showDetails={true}
          onReconnect={handleReconnect}
        />
      );

      const button = screen.getByRole('button', { name: 'Reconnect' });
      expect(button).toBeInTheDocument();
    });

    it('应该显示重连按钮（error 状态）', () => {
      const handleReconnect = jest.fn();
      render(
        <ConnectionStatusIndicator
          status={'error' as ConnectionStatus}
          showDetails={true}
          onReconnect={handleReconnect}
        />
      );

      const button = screen.getByRole('button', { name: 'Reconnect' });
      expect(button).toBeInTheDocument();
    });

    it('connected 状态不应显示重连按钮', () => {
      const handleReconnect = jest.fn();
      render(
        <ConnectionStatusIndicator
          status={'connected' as ConnectionStatus}
          showDetails={true}
          onReconnect={handleReconnect}
        />
      );

      expect(screen.queryByRole('button', { name: 'Reconnect' })).not.toBeInTheDocument();
    });
  });

  describe('交互行为', () => {
    it('应该响应重连按钮点击', async () => {
      const handleReconnect = jest.fn();
      render(
        <ConnectionStatusIndicator
          status={'disconnected' as ConnectionStatus}
          showDetails={true}
          onReconnect={handleReconnect}
        />
      );

      const button = screen.getByRole('button', { name: 'Reconnect' });
      await userEvent.click(button);

      expect(handleReconnect).toHaveBeenCalledTimes(1);
    });
  });
});

describe('ConnectionDot', () => {
  describe('渲染测试', () => {
    it('应该渲染 connected 状态点', () => {
      const { container } = render(
        <ConnectionDot status={'connected' as ConnectionStatus} />
      );
      const dot = container.firstChild as HTMLElement;

      expect(dot).toBeInTheDocument();
      expect(dot).toHaveClass('bg-green-500');
    });

    it('应该渲染不同大小的点', () => {
      const { container: containerSm } = render(
        <ConnectionDot status={'connected' as ConnectionStatus} size="sm" />
      );
      const { container: containerMd } = render(
        <ConnectionDot status={'connected' as ConnectionStatus} size="md" />
      );
      const { container: containerLg } = render(
        <ConnectionDot status={'connected' as ConnectionStatus} size="lg" />
      );

      expect((containerSm.firstChild as HTMLElement)).toHaveClass('w-2', 'h-2');
      expect((containerMd.firstChild as HTMLElement)).toHaveClass('w-3', 'h-3');
      expect((containerLg.firstChild as HTMLElement)).toHaveClass('w-4', 'h-4');
    });

    it('应该有自定义 title', () => {
      render(
        <ConnectionDot
          status={'connected' as ConnectionStatus}
          title="Custom title"
        />
      );

      const dot = document.querySelector('[title="Custom title"]');
      expect(dot).toBeInTheDocument();
    });
  });

  describe('动画效果', () => {
    it('connecting 状态应该有脉冲动画', () => {
      const { container } = render(
        <ConnectionDot status={'connecting' as ConnectionStatus} />
      );
      const dot = container.firstChild as HTMLElement;

      expect(dot).toHaveClass('bg-yellow-500', 'animate-pulse');
    });

    it('reconnecting 状态应该有脉冲动画', () => {
      const { container } = render(
        <ConnectionDot status={'reconnecting' as ConnectionStatus} />
      );
      const dot = container.firstChild as HTMLElement;

      expect(dot).toHaveClass('bg-orange-500', 'animate-pulse');
    });
  });
});

describe('ConnectionToast', () => {
  describe('显示条件', () => {
    it('connected 状态不应显示 toast', () => {
      const { container } = render(
        <ConnectionToast status={'connected' as ConnectionStatus} />
      );

      expect(container.firstChild).toBe(null);
    });

    it('connecting 状态不应显示 toast', () => {
      const { container } = render(
        <ConnectionToast status={'connecting' as ConnectionStatus} />
      );

      expect(container.firstChild).toBe(null);
    });

    it('disconnected 状态应该显示 toast', () => {
      render(
        <ConnectionToast status={'disconnected' as ConnectionStatus} />
      );

      expect(screen.getByText('Disconnected')).toBeInTheDocument();
    });

    it('error 状态应该显示 toast', () => {
      render(
        <ConnectionToast status={'error' as ConnectionStatus} error="Connection failed" />
      );

      expect(screen.getByText('Error')).toBeInTheDocument();
      expect(screen.getByText('Connection failed')).toBeInTheDocument();
    });
  });

  describe('交互行为', () => {
    it('应该响应关闭按钮点击', async () => {
      const handleDismiss = jest.fn();
      render(
        <ConnectionToast
          status={'disconnected' as ConnectionStatus}
          onDismiss={handleDismiss}
        />
      );

      const button = screen.getByRole('button', { name: /✕/i });
      await userEvent.click(button);

      expect(handleDismiss).toHaveBeenCalledTimes(1);
    });
  });
});
