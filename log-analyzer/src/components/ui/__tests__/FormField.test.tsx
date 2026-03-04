/**
 * FormField 组件单元测试
 *
 * 测试表单字段包装器的标签、错误显示和无障碍功能
 */

import { render, screen } from '@testing-library/react';
import { FormField, FormErrorSummary, FormGroup } from '../FormField';
import { Input } from '../Input';

// 创建一个测试用的 Input 组件（如果需要）
const TestInput = ({ id, ...props }: any) => (
  <input id={id} data-testid={id} {...props} />
);

describe('FormField', () => {
  describe('渲染测试', () => {
    it('应该渲染标签', () => {
      render(
        <FormField label="Username">
          <TestInput />
        </FormField>
      );

      expect(screen.getByLabelText('Username')).toBeInTheDocument();
    });

    it('应该渲染必填标记', () => {
      render(
        <FormField label="Email" required>
          <TestInput />
        </FormField>
      );

      const label = screen.getByText('Email');
      expect(label).toBeInTheDocument();
      // 检查星号存在
      expect(label.innerHTML).toContain('*');
    });

    it('应该渲染描述文本', () => {
      render(
        <FormField label="Password" description="At least 8 characters">
          <TestInput />
        </FormField>
      );

      expect(screen.getByText('At least 8 characters')).toBeInTheDocument();
    });

    it('应该渲染单个错误消息', () => {
      render(
        <FormField label="Email" error="Invalid email format">
          <TestInput />
        </FormField>
      );

      expect(screen.getByText('Invalid email format')).toBeInTheDocument();
    });

    it('应该渲染多个错误消息', () => {
      render(
        <FormField
          label="Password"
          error={['Too short', 'Must contain numbers']}
        >
          <TestInput />
        </FormField>
      );

      expect(screen.getByText('Too short')).toBeInTheDocument();
      expect(screen.getByText('Must contain numbers')).toBeInTheDocument();
    });
  });

  describe('无障碍性', () => {
    it('应该关联标签和输入框', () => {
      render(
        <FormField label="Username">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Username');
      expect(input).toBeInstanceOf(HTMLInputElement);
    });

    it('应该在有错误时设置 aria-invalid', () => {
      render(
        <FormField label="Email" error="Required">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Email');
      expect(input).toHaveAttribute('aria-invalid', 'true');
    });

    it('应该设置 aria-describedby 到描述文本', () => {
      render(
        <FormField label="Password" description="At least 8 characters">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Password');
      const description = screen.getByText('At least 8 characters');

      expect(input).toHaveAttribute('aria-describedby', description.id);
    });

    it('应该设置 aria-describedby 到错误消息', () => {
      render(
        <FormField label="Email" error="Invalid">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Email');
      const error = screen.getByRole('alert');

      expect(input).toHaveAttribute('aria-describedby', error.id);
    });

    it('错误状态应该有正确的角色', () => {
      render(
        <FormField label="Email" error="Invalid">
          <TestInput />
        </FormField>
      );

      const error = screen.getByRole('alert');
      expect(error).toBeInTheDocument();
      expect(error).toHaveAttribute('aria-live', 'polite');
    });
  });

  describe('样式变化', () => {
    it('错误状态应该改变标签颜色', () => {
      render(
        <FormField label="Email" error="Required">
          <TestInput />
        </FormField>
      );

      const label = screen.getByText('Email');
      expect(label).toHaveClass('text-red-600');
    });

    it('错误状态应该给输入框添加红色边框样式', () => {
      render(
        <FormField label="Email" error="Required">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Email') as HTMLInputElement;
      // 组件应该通过 cloneElement 添加 border-red-500 类
      expect(input).toHaveClass('border-red-500');
    });
  });

  describe('自定义 ID', () => {
    it('应该使用提供的 ID', () => {
      render(
        <FormField label="Email" id="custom-id">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Email');
      expect(input.id).toBe('custom-id');
    });

    it('应该基于自定义 ID 生成错误 ID', () => {
      render(
        <FormField label="Email" id="custom-id" error="Invalid">
          <TestInput />
        </FormField>
      );

      const input = screen.getByLabelText('Email');
      const error = screen.getByRole('alert');

      expect(input).toHaveAttribute('aria-describedby', 'custom-id-error');
      expect(error.id).toBe('custom-id-error');
    });
  });

  describe('自定义类名', () => {
    it('应该支持自定义类名', () => {
      render(
        <FormField label="Email" className="mb-4">
          <TestInput />
        </FormField>
      );

      const wrapper = screen.getByLabelText('Email').closest('.mb-4');
      expect(wrapper).toBeInTheDocument();
    });
  });
});

describe('FormGroup', () => {
  describe('渲染测试', () => {
    it('应该渲染标题', () => {
      render(
        <FormGroup title="User Information">
          <FormField label="Name"><TestInput /></FormField>
        </FormGroup>
      );

      expect(screen.getByText('User Information')).toBeInTheDocument();
      expect(screen.getByRole('group')).toBeInTheDocument();
    });

    it('应该渲染描述', () => {
      render(
        <FormGroup title="Contact" description="Your contact details">
          <FormField label="Email"><TestInput /></FormField>
        </FormGroup>
      );

      expect(screen.getByText('Your contact details')).toBeInTheDocument();
    });

    it('应该渲染多个子元素', () => {
      render(
        <FormGroup title="Account">
          <FormField label="Username"><TestInput /></FormField>
          <FormField label="Password"><TestInput /></FormField>
        </FormGroup>
      );

      expect(screen.getByLabelText('Username')).toBeInTheDocument();
      expect(screen.getByLabelText('Password')).toBeInTheDocument();
    });
  });

  describe('无障碍性', () => {
    it('应该使用 fieldset 元素', () => {
      render(
        <FormGroup title="Personal Info">
          <FormField label="Name"><TestInput /></FormField>
        </FormGroup>
      );

      const fieldset = screen.getByRole('group');
      expect(fieldset).toBeInTheDocument();
      expect(fieldset.tagName).toBe('FIELDSET');
    });

    it('标题应该使用 legend 元素', () => {
      render(
        <FormGroup title="Contact Info">
          <FormField label="Email"><TestInput /></FormField>
        </FormGroup>
      );

      const legend = screen.getByText('Contact Info');
      expect(legend.tagName).toBe('LEGEND');
    });
  });
});

describe('FormErrorSummary', () => {
  describe('渲染测试', () => {
    it('空错误对象不应渲染任何内容', () => {
      const { container } = render(
        <FormErrorSummary errors={{}} />
      );

      expect(container.firstChild).toBe(null);
    });

    it('应该渲染错误摘要', () => {
      render(
        <FormErrorSummary
          errors={{
            email: 'Invalid email format',
            password: 'Too short',
          }}
        />
      );

      expect(screen.getByText(/please correct/i)).toBeInTheDocument();
      // Jest 环境中 Tailwind capitalize 类不生效，所以使用不区分大小写的匹配
      expect(screen.getByText(/email: Invalid email format/i)).toBeInTheDocument();
      expect(screen.getByText(/password: Too short/i)).toBeInTheDocument();
    });

    it('应该处理数组错误消息', () => {
      render(
        <FormErrorSummary
          errors={{
            password: ['Too short', 'Must contain uppercase', 'Must contain numbers'],
          }}
        />
      );

      // 使用不区分大小写的匹配，因为 Jest 环境中 Tailwind 的 capitalize 不生效
      expect(screen.getByText(/password: Too short/i)).toBeInTheDocument();
      expect(screen.getByText(/password: Must contain uppercase/i)).toBeInTheDocument();
      expect(screen.getByText(/password: Must contain numbers/i)).toBeInTheDocument();
    });

    it('应该支持自定义标题', () => {
      render(
        <FormErrorSummary
          errors={{ email: 'Invalid' }}
          title="Fix these issues"
        />
      );

      expect(screen.getByText('Fix these issues')).toBeInTheDocument();
    });
  });

  describe('无障碍性', () => {
    it('应该有正确的 alert 角色', () => {
      render(
        <FormErrorSummary
          errors={{ email: 'Invalid format' }}
        />
      );

      const alert = screen.getByRole('alert');
      expect(alert).toBeInTheDocument();
      expect(alert).toHaveAttribute('aria-live', 'polite');
    });

    it('应该使用列表格式展示错误', () => {
      render(
        <FormErrorSummary
          errors={{ email: 'Invalid' }}
        />
      );

      const list = screen.getByRole('list');
      expect(list).toBeInTheDocument();
    });
  });

  describe('样式类', () => {
    it('应该有基础错误样式类', () => {
      render(
        <FormErrorSummary
          errors={{ email: 'Invalid' }}
        />
      );

      const alert = screen.getByRole('alert');
      expect(alert).toHaveClass('bg-red-50');
    });
  });
});
