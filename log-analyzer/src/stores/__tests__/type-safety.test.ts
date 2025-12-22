/**
 * 类型安全测试
 * 
 * 验证所有 store 模块的类型完整性和类型安全
 * 
 * Property 32: Store Type Completeness - 验证所有 store 类型完整性
 * Property 33: Hook Type Safety - 验证 hooks 类型推断正确
 * Property 34: Action Method Availability - 验证所有 action 方法可访问
 * Property 35: Utility Function Completeness - 验证工具函数类型完整
 * 
 * Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5
 */

import { describe, it, expect } from '@jest/globals';
import { useAppStore } from '../appStore';
import { useWorkspaceStore, type Workspace } from '../workspaceStore';
import { useTaskStore, type Task } from '../taskStore';
import { useKeywordStore, type KeywordGroup } from '../keywordStore';

// Mock logger to avoid import.meta issues in Jest
jest.mock('../../utils/logger');
import { logger } from '../../utils/logger';

describe('Type Safety Tests', () => {
  describe('Property 32: Store Type Completeness', () => {
    it('appStore 应该有完整的类型定义', () => {
      const store = useAppStore.getState();
      
      // 验证状态属性存在且类型正确
      expect(typeof store.page).toBe('string');
      expect(Array.isArray(store.toasts)).toBe(true);
      expect(store.activeWorkspaceId === null || typeof store.activeWorkspaceId === 'string').toBe(true);
      
      // 验证 action 方法存在且类型正确
      expect(typeof store.setPage).toBe('function');
      expect(typeof store.addToast).toBe('function');
      expect(typeof store.removeToast).toBe('function');
      expect(typeof store.setActiveWorkspace).toBe('function');
    });

    it('workspaceStore 应该有完整的类型定义', () => {
      const store = useWorkspaceStore.getState();
      
      // 验证状态属性存在且类型正确
      expect(Array.isArray(store.workspaces)).toBe(true);
      expect(typeof store.loading).toBe('boolean');
      expect(store.error === null || typeof store.error === 'string').toBe(true);
      
      // 验证 action 方法存在且类型正确
      expect(typeof store.setWorkspaces).toBe('function');
      expect(typeof store.addWorkspace).toBe('function');
      expect(typeof store.updateWorkspace).toBe('function');
      expect(typeof store.deleteWorkspace).toBe('function');
      expect(typeof store.setLoading).toBe('function');
      expect(typeof store.setError).toBe('function');
    });

    it('taskStore 应该有完整的类型定义', () => {
      const store = useTaskStore.getState();
      
      // 验证状态属性存在且类型正确
      expect(Array.isArray(store.tasks)).toBe(true);
      expect(typeof store.loading).toBe('boolean');
      expect(store.error === null || typeof store.error === 'string').toBe(true);
      
      // 验证 action 方法存在且类型正确
      expect(typeof store.setTasks).toBe('function');
      expect(typeof store.addTask).toBe('function');
      expect(typeof store.addTaskIfNotExists).toBe('function');
      expect(typeof store.updateTask).toBe('function');
      expect(typeof store.deleteTask).toBe('function');
      expect(typeof store.setLoading).toBe('function');
      expect(typeof store.setError).toBe('function');
    });

    it('keywordStore 应该有完整的类型定义', () => {
      const store = useKeywordStore.getState();
      
      // 验证状态属性存在且类型正确
      expect(Array.isArray(store.keywordGroups)).toBe(true);
      expect(typeof store.loading).toBe('boolean');
      expect(store.error === null || typeof store.error === 'string').toBe(true);
      
      // 验证 action 方法存在且类型正确
      expect(typeof store.setKeywordGroups).toBe('function');
      expect(typeof store.addKeywordGroup).toBe('function');
      expect(typeof store.updateKeywordGroup).toBe('function');
      expect(typeof store.deleteKeywordGroup).toBe('function');
      expect(typeof store.toggleKeywordGroup).toBe('function');
      expect(typeof store.setLoading).toBe('function');
      expect(typeof store.setError).toBe('function');
    });
  });

  describe('Property 33: Hook Type Safety', () => {
    it('useAppStore 应该有正确的类型定义', () => {
      // 直接测试 store 的类型定义，而不是在非 React 环境中使用 hooks
      const store = useAppStore.getState();
      
      // 验证状态类型
      const page: string = store.page;
      const toasts: any[] = store.toasts;
      const activeWorkspaceId: string | null = store.activeWorkspaceId;
      
      // 验证类型推断正确
      expect(typeof page).toBe('string');
      expect(Array.isArray(toasts)).toBe(true);
      expect(activeWorkspaceId === null || typeof activeWorkspaceId === 'string').toBe(true);
    });

    it('useWorkspaceStore 应该有正确的类型定义', () => {
      const store = useWorkspaceStore.getState();
      
      // 验证状态类型
      const workspaces: Workspace[] = store.workspaces;
      const loading: boolean = store.loading;
      
      expect(Array.isArray(workspaces)).toBe(true);
      expect(typeof loading).toBe('boolean');
    });

    it('useTaskStore 应该有正确的类型定义', () => {
      const store = useTaskStore.getState();
      
      // 验证状态类型
      const tasks: Task[] = store.tasks;
      const loading: boolean = store.loading;
      
      expect(Array.isArray(tasks)).toBe(true);
      expect(typeof loading).toBe('boolean');
    });

    it('useKeywordStore 应该有正确的类型定义', () => {
      const store = useKeywordStore.getState();
      
      // 验证状态类型
      const keywordGroups: KeywordGroup[] = store.keywordGroups;
      const loading: boolean = store.loading;
      
      expect(Array.isArray(keywordGroups)).toBe(true);
      expect(typeof loading).toBe('boolean');
    });
  });

  describe('Property 34: Action Method Availability', () => {
    beforeEach(() => {
      // 在每个测试前重置所有 store 状态
      useAppStore.setState({
        page: 'workspaces',
        toasts: [],
        activeWorkspaceId: null
      });
      
      useWorkspaceStore.setState({
        workspaces: [],
        loading: false,
        error: null
      });
      
      useTaskStore.setState({
        tasks: [],
        loading: false,
        error: null
      });
      
      useKeywordStore.setState({
        keywordGroups: [],
        loading: false,
        error: null
      });
    });

    it('appStore 的所有 action 方法应该可访问', () => {
      const store = useAppStore.getState();
      
      // 测试 setPage
      store.setPage('workspaces');
      expect(useAppStore.getState().page).toBe('workspaces');
      
      // 测试 setActiveWorkspace
      store.setActiveWorkspace('test-workspace');
      expect(useAppStore.getState().activeWorkspaceId).toBe('test-workspace');
      
      store.setActiveWorkspace(null);
      expect(useAppStore.getState().activeWorkspaceId).toBe(null);
    });

    it('workspaceStore 的所有 action 方法应该可访问', () => {
      const store = useWorkspaceStore.getState();
      
      // 测试 addWorkspace
      const testWorkspace: Workspace = {
        id: 'test-1',
        name: 'Test Workspace',
        path: '/test/path',
        status: 'READY',
        size: '100MB',
        files: 10
      };
      
      store.addWorkspace(testWorkspace);
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(1);
      expect(useWorkspaceStore.getState().workspaces[0].id).toBe('test-1');
      
      // 测试 updateWorkspace
      store.updateWorkspace('test-1', { status: 'SCANNING' });
      expect(useWorkspaceStore.getState().workspaces[0].status).toBe('SCANNING');
      
      // 测试 deleteWorkspace
      store.deleteWorkspace('test-1');
      expect(useWorkspaceStore.getState().workspaces).toHaveLength(0);
      
      // 测试 setLoading 和 setError
      store.setLoading(true);
      expect(useWorkspaceStore.getState().loading).toBe(true);
      
      store.setError('Test error');
      expect(useWorkspaceStore.getState().error).toBe('Test error');
    });

    it('taskStore 的所有 action 方法应该可访问', () => {
      const store = useTaskStore.getState();
      
      // 测试 addTask
      const testTask: Task = {
        id: 'task-1',
        type: 'import',
        target: 'test.log',
        progress: 0,
        message: 'Starting',
        status: 'RUNNING'
      };
      
      store.addTask(testTask);
      expect(useTaskStore.getState().tasks).toHaveLength(1);
      
      // 测试 addTaskIfNotExists（去重）
      store.addTaskIfNotExists(testTask);
      expect(useTaskStore.getState().tasks).toHaveLength(1); // 不应该重复添加
      
      // 测试 updateTask
      store.updateTask('task-1', { progress: 50, message: 'Processing' });
      expect(useTaskStore.getState().tasks[0].progress).toBe(50);
      expect(useTaskStore.getState().tasks[0].message).toBe('Processing');
      
      // 测试 deleteTask
      store.deleteTask('task-1');
      expect(useTaskStore.getState().tasks).toHaveLength(0);
    });

    it('keywordStore 的所有 action 方法应该可访问', () => {
      const store = useKeywordStore.getState();
      
      // 测试 addKeywordGroup
      const testGroup: KeywordGroup = {
        id: 'group-1',
        name: 'Test Group',
        color: 'blue',
        patterns: [{ regex: 'test', comment: 'Test pattern' }],
        enabled: true
      };
      
      store.addKeywordGroup(testGroup);
      expect(useKeywordStore.getState().keywordGroups).toHaveLength(1);
      
      // 测试 toggleKeywordGroup
      store.toggleKeywordGroup('group-1');
      expect(useKeywordStore.getState().keywordGroups[0].enabled).toBe(false);
      
      // 测试 updateKeywordGroup
      const updatedGroup = { ...testGroup, name: 'Updated Group', enabled: true };
      store.updateKeywordGroup(updatedGroup);
      expect(useKeywordStore.getState().keywordGroups[0].name).toBe('Updated Group');
      
      // 测试 deleteKeywordGroup
      store.deleteKeywordGroup('group-1');
      expect(useKeywordStore.getState().keywordGroups).toHaveLength(0);
    });
  });

  describe('Property 35: Utility Function Completeness', () => {
    it('logger 工具应该有完整的方法', () => {
      // 验证所有日志方法存在
      expect(typeof logger.debug).toBe('function');
      expect(typeof logger.info).toBe('function');
      expect(typeof logger.warn).toBe('function');
      expect(typeof logger.error).toBe('function');
      
      // 测试方法可以正常调用（不会抛出错误）
      expect(() => logger.debug('Test debug')).not.toThrow();
      expect(() => logger.info('Test info')).not.toThrow();
      expect(() => logger.warn('Test warn')).not.toThrow();
      expect(() => logger.error('Test error')).not.toThrow();
    });

    it('logger 方法应该接受额外参数', () => {
      // 测试方法可以接受多个参数
      expect(() => logger.debug('Test', { data: 'value' }, 123)).not.toThrow();
      expect(() => logger.info('Test', ['array'], true)).not.toThrow();
      expect(() => logger.warn('Test', new Error('test'))).not.toThrow();
      expect(() => logger.error('Test', null, undefined)).not.toThrow();
    });
  });

  describe('Type Exports', () => {
    it('所有类型应该正确导出', () => {
      // 验证类型可以被导入和使用
      const workspace: Workspace = {
        id: 'test',
        name: 'Test',
        path: '/test',
        status: 'READY',
        size: '0',
        files: 0
      };
      
      const task: Task = {
        id: 'test',
        type: 'import',
        target: 'test',
        progress: 0,
        message: 'test',
        status: 'RUNNING'
      };
      
      const keywordGroup: KeywordGroup = {
        id: 'test',
        name: 'Test',
        color: 'blue',
        patterns: [],
        enabled: true
      };
      
      // 如果类型定义不完整，这些赋值会导致 TypeScript 编译错误
      expect(workspace.id).toBe('test');
      expect(task.id).toBe('test');
      expect(keywordGroup.id).toBe('test');
    });
  });
});
