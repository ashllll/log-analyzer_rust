/**
 * API 层单元测试
 *
 * 测试统一 API 层的 Tauri 命令调用和错误处理
 */

// Mock Tauri invoke before importing api
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { api, SearchParams, ExportParams } from '../api';
import { createApiError } from '../errors';

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

describe('API 层测试', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  // ========================================================================
  // 工作区操作测试
  // ========================================================================

  describe('工作区操作', () => {
    describe('loadWorkspace', () => {
      it('应该成功加载工作区', async () => {
        const mockResponse = {
          id: 'ws-123',
          name: 'Test Workspace',
          path: '/test/path',
          status: 'READY' as const,
          fileCount: 100,
          totalSize: 1024000,
        };
        mockInvoke.mockResolvedValue(mockResponse);

        const result = await api.loadWorkspace('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('load_workspace', {
          workspaceId: 'ws-123',
        });
        expect(result).toEqual(mockResponse);
      });

      it('应该处理加载工作区错误', async () => {
        const error = new Error('Workspace not found');
        mockInvoke.mockRejectedValue(error);

        await expect(api.loadWorkspace('ws-123')).rejects.toThrow();
      });

      it('应该将错误转换为 ApiError', async () => {
        const originalError = { code: 'NOT_FOUND', message: 'Workspace not found' };
        mockInvoke.mockRejectedValue(JSON.stringify(originalError));

        try {
          await api.loadWorkspace('ws-123');
        } catch (error) {
          expect(error).toBeInstanceOf(Error);
          if (error instanceof Error) {
            expect(error.message).toContain('Workspace not found');
          }
        }
      });
    });

    describe('refreshWorkspace', () => {
      it('应该成功刷新工作区', async () => {
        mockInvoke.mockResolvedValue('ws-123');

        const result = await api.refreshWorkspace('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('refresh_workspace', {
          workspaceId: 'ws-123',
        });
        expect(result).toBe('ws-123');
      });

      it('应该处理刷新错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Refresh failed'));

        await expect(api.refreshWorkspace('ws-123')).rejects.toThrow();
      });
    });

    describe('deleteWorkspace', () => {
      it('应该成功删除工作区', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.deleteWorkspace('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('delete_workspace', {
          workspaceId: 'ws-123',
        });
      });

      it('应该处理删除错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Delete failed'));

        await expect(api.deleteWorkspace('ws-123')).rejects.toThrow();
      });
    });

    describe('getWorkspaceStatus', () => {
      it('应该成功获取工作区状态', async () => {
        const mockStatus = {
          id: 'ws-123',
          name: 'Test',
          status: 'READY' as const,
          fileCount: 50,
          totalSize: 512000,
          watching: true,
        };
        mockInvoke.mockResolvedValue(mockStatus);

        const result = await api.getWorkspaceStatus('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('get_workspace_status', {
          workspaceId: 'ws-123',
        });
        expect(result).toEqual(mockStatus);
      });

      it('应该处理状态获取错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Status unavailable'));

        await expect(api.getWorkspaceStatus('ws-123')).rejects.toThrow();
      });
    });

    describe('createWorkspace', () => {
      it('应该成功创建工作区', async () => {
        mockInvoke.mockResolvedValue('ws-new');

        const result = await api.createWorkspace('New Workspace', '/new/path');

        expect(mockInvoke).toHaveBeenCalledWith('create_workspace', {
          name: 'New Workspace',
          path: '/new/path',
        });
        expect(result).toBe('ws-new');
      });

      it('应该处理创建错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Creation failed'));

        await expect(
          api.createWorkspace('New', '/path')
        ).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 搜索操作测试
  // ========================================================================

  describe('搜索操作', () => {
    describe('searchLogs', () => {
      it('应该成功执行搜索', async () => {
        mockInvoke.mockResolvedValue('search-123');

        const params: SearchParams = {
          query: 'error timeout',
          workspaceId: 'ws-123',
          maxResults: 1000,
        };

        const result = await api.searchLogs(params);

        expect(mockInvoke).toHaveBeenCalledWith('search_logs', params);
        expect(result).toBe('search-123');
      });

      it('应该支持搜索过滤器', async () => {
        mockInvoke.mockResolvedValue('search-456');

        const params: SearchParams = {
          query: 'error',
          workspaceId: 'ws-123',
          filters: {
            levels: ['ERROR', 'WARN'],
            timeRange: {
              start: '2024-01-01T00:00:00Z',
              end: '2024-01-02T00:00:00Z',
            },
            filePattern: '*.log',
          },
        };

        const result = await api.searchLogs(params);

        expect(mockInvoke).toHaveBeenCalledWith('search_logs', params);
        expect(result).toBe('search-456');
      });

      it('应该处理搜索错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Search failed'));

        const params: SearchParams = {
          query: 'test',
        };

        await expect(api.searchLogs(params)).rejects.toThrow();
      });
    });

    describe('cancelSearch', () => {
      it('应该成功取消搜索', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.cancelSearch('search-123');

        expect(mockInvoke).toHaveBeenCalledWith('cancel_search', {
          searchId: 'search-123',
        });
      });

      it('应该处理取消错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Cancel failed'));

        await expect(api.cancelSearch('search-123')).rejects.toThrow();
      });
    });

    describe('asyncSearchLogs', () => {
      it('应该成功执行异步搜索', async () => {
        mockInvoke.mockResolvedValue('async-search-123');

        const params: SearchParams = {
          query: 'async test',
          workspaceId: 'ws-123',
        };

        const result = await api.asyncSearchLogs(params);

        expect(mockInvoke).toHaveBeenCalledWith('async_search_logs', params);
        expect(result).toBe('async-search-123');
      });

      it('应该处理异步搜索错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Async search failed'));

        await expect(
          api.asyncSearchLogs({ query: 'test' })
        ).rejects.toThrow();
      });
    });

    describe('cancelAsyncSearch', () => {
      it('应该成功取消异步搜索', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.cancelAsyncSearch('async-search-123');

        expect(mockInvoke).toHaveBeenCalledWith('cancel_async_search', {
          searchId: 'async-search-123',
        });
      });
    });
  });

  // ========================================================================
  // 导入操作测试
  // ========================================================================

  describe('导入操作', () => {
    describe('importFolder', () => {
      it('应该成功导入文件夹', async () => {
        mockInvoke.mockResolvedValue('task-123');

        const result = await api.importFolder('/test/path', 'ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('import_folder', {
          path: '/test/path',
          workspaceId: 'ws-123',
        });
        expect(result).toBe('task-123');
      });

      it('应该处理导入错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Import failed'));

        await expect(
          api.importFolder('/test/path', 'ws-123')
        ).rejects.toThrow();
      });
    });

    describe('checkRarSupport', () => {
      it('应该成功检查 RAR 支持', async () => {
        const mockSupport = {
          enabled: true,
          version: '6.0.0',
        };
        mockInvoke.mockResolvedValue(mockSupport);

        const result = await api.checkRarSupport();

        expect(mockInvoke).toHaveBeenCalledWith('check_rar_support');
        expect(result).toEqual(mockSupport);
      });

      it('应该处理 RAR 支持检查错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Check failed'));

        await expect(api.checkRarSupport()).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 文件监听测试
  // ========================================================================

  describe('文件监听', () => {
    describe('startWatch', () => {
      it('应该成功启动文件监听', async () => {
        mockInvoke.mockResolvedValue(undefined);

        const params = {
          workspaceId: 'ws-123',
          autoSearch: true,
        };

        await api.startWatch(params);

        expect(mockInvoke).toHaveBeenCalledWith('start_watch', params);
      });

      it('应该处理启动监听错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Start failed'));

        await expect(
          api.startWatch({ workspaceId: 'ws-123' })
        ).rejects.toThrow();
      });
    });

    describe('stopWatch', () => {
      it('应该成功停止文件监听', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.stopWatch('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('stop_watch', {
          workspaceId: 'ws-123',
        });
      });

      it('应该处理停止监听错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Stop failed'));

        await expect(api.stopWatch('ws-123')).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 任务管理测试
  // ========================================================================

  describe('任务管理', () => {
    describe('cancelTask', () => {
      it('应该成功取消任务', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.cancelTask('task-123');

        expect(mockInvoke).toHaveBeenCalledWith('cancel_task', {
          taskId: 'task-123',
        });
      });

      it('应该处理取消任务错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Cancel failed'));

        await expect(api.cancelTask('task-123')).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 配置管理测试
  // ========================================================================

  describe('配置管理', () => {
    describe('saveConfig', () => {
      it('应该成功保存配置', async () => {
        mockInvoke.mockResolvedValue(undefined);

        const config = {
          keyword_groups: [],
          workspaces: [],
          advanced_features: {
            enable_filter_engine: true,
            enable_regex_engine: true,
            enable_time_partition: false,
            enable_autocomplete: true,
            regex_cache_size: 1000,
            autocomplete_limit: 100,
            time_partition_size_secs: 3600,
          },
          file_filter: {
            enabled: false,
            binary_detection_enabled: true,
            mode: 'whitelist' as const,
            filename_patterns: [],
            allowed_extensions: [],
            forbidden_extensions: [],
          },
        };

        await api.saveConfig(config);

        expect(mockInvoke).toHaveBeenCalledWith('save_config', { config });
      });

      it('应该处理保存配置错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Save failed'));

        await expect(
          api.saveConfig({
            keyword_groups: [],
            workspaces: [],
            advanced_features: {
              enable_filter_engine: true,
              enable_regex_engine: true,
              enable_time_partition: false,
              enable_autocomplete: true,
              regex_cache_size: 1000,
              autocomplete_limit: 100,
              time_partition_size_secs: 3600,
            },
            file_filter: {
              enabled: false,
              binary_detection_enabled: true,
              mode: 'whitelist',
              filename_patterns: [],
              allowed_extensions: [],
              forbidden_extensions: [],
            },
          })
        ).rejects.toThrow();
      });
    });

    describe('loadConfig', () => {
      it('应该成功加载配置', async () => {
        const mockConfig = {
          keyword_groups: [{ id: 'kg-1', name: 'Test' }],
          workspaces: [],
          advanced_features: {
            enable_filter_engine: true,
            enable_regex_engine: true,
            enable_time_partition: false,
            enable_autocomplete: true,
            regex_cache_size: 1000,
            autocomplete_limit: 100,
            time_partition_size_secs: 3600,
          },
          file_filter: {
            enabled: false,
            binary_detection_enabled: true,
            mode: 'whitelist' as const,
            filename_patterns: [],
            allowed_extensions: [],
            forbidden_extensions: [],
          },
        };
        mockInvoke.mockResolvedValue(mockConfig);

        const result = await api.loadConfig();

        expect(mockInvoke).toHaveBeenCalledWith('load_config');
        expect(result).toEqual(mockConfig);
      });

      it('应该处理加载配置错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Load failed'));

        await expect(api.loadConfig()).rejects.toThrow();
      });
    });

    describe('getFileFilterConfig', () => {
      it('应该成功获取文件过滤器配置', async () => {
        const mockConfig = {
          enabled: true,
          binary_detection_enabled: true,
          mode: 'whitelist',
          filename_patterns: ['*.log'],
        };
        mockInvoke.mockResolvedValue(mockConfig);

        const result = await api.getFileFilterConfig();

        expect(mockInvoke).toHaveBeenCalledWith('get_file_filter_config');
        expect(result).toEqual(mockConfig);
      });

      it('应该处理获取配置错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Get failed'));

        await expect(api.getFileFilterConfig()).rejects.toThrow();
      });
    });

    describe('saveFileFilterConfig', () => {
      it('应该成功保存文件过滤器配置', async () => {
        mockInvoke.mockResolvedValue(undefined);

        const filterConfig = {
          enabled: true,
          mode: 'blacklist',
          filename_patterns: ['*.tmp'],
        };

        await api.saveFileFilterConfig(filterConfig);

        expect(mockInvoke).toHaveBeenCalledWith('save_file_filter_config', {
          filterConfig,
        });
      });

      it('应该处理保存错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Save failed'));

        await expect(
          api.saveFileFilterConfig({ enabled: false })
        ).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 性能监控测试
  // ========================================================================

  describe('性能监控', () => {
    describe('getPerformanceMetrics', () => {
      it('应该成功获取性能指标', async () => {
        const mockMetrics = {
          searchCount: 100,
          averageLatency: 45,
          cacheHitRate: 0.85,
        };
        mockInvoke.mockResolvedValue(mockMetrics);

        const result = await api.getPerformanceMetrics();

        expect(mockInvoke).toHaveBeenCalledWith('get_performance_metrics');
        expect(result).toEqual(mockMetrics);
      });

      it('应该处理获取指标错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Metrics unavailable'));

        await expect(api.getPerformanceMetrics()).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 导出操作测试
  // ========================================================================

  describe('导出操作', () => {
    describe('exportResults', () => {
      it('应该成功导出为 CSV', async () => {
        mockInvoke.mockResolvedValue('/exported/results.csv');

        const params: ExportParams = {
          results: [{ content: 'test' }],
          format: 'csv',
          savePath: '/exported/results.csv',
        };

        const result = await api.exportResults(params);

        expect(mockInvoke).toHaveBeenCalledWith('export_results', params);
        expect(result).toBe('/exported/results.csv');
      });

      it('应该成功导出为 JSON', async () => {
        mockInvoke.mockResolvedValue('/exported/results.json');

        const params: ExportParams = {
          results: [{ content: 'test' }],
          format: 'json',
          savePath: '/exported/results.json',
        };

        const result = await api.exportResults(params);

        expect(result).toBe('/exported/results.json');
      });

      it('应该处理导出错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Export failed'));

        await expect(
          api.exportResults({
            results: [],
            format: 'csv',
            savePath: '/test.csv',
          })
        ).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 虚拟文件树测试
  // ========================================================================

  describe('虚拟文件树', () => {
    describe('readFileByHash', () => {
      it('应该成功通过哈希读取文件', async () => {
        const mockResponse = {
          content: 'file content',
          encoding: 'utf-8',
        };
        mockInvoke.mockResolvedValue(mockResponse);

        const params = {
          workspaceId: 'ws-123',
          hash: 'abc123',
          maxLength: 10000,
        };

        const result = await api.readFileByHash(params);

        expect(mockInvoke).toHaveBeenCalledWith('read_file_by_hash', params);
        expect(result).toEqual(mockResponse);
      });

      it('应该支持不带 maxLength 的参数', async () => {
        mockInvoke.mockResolvedValue({ content: 'test' });

        const params = {
          workspaceId: 'ws-123',
          hash: 'abc123',
        };

        await api.readFileByHash(params);

        expect(mockInvoke).toHaveBeenCalledWith('read_file_by_hash', params);
      });

      it('应该处理读取错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Read failed'));

        await expect(
          api.readFileByHash({ workspaceId: 'ws-123', hash: 'abc123' })
        ).rejects.toThrow();
      });
    });

    describe('getVirtualFileTree', () => {
      it('应该成功获取虚拟文件树', async () => {
        const mockTree = [
          { id: '1', name: 'file1.log', type: 'file' },
          { id: '2', name: 'folder', type: 'directory' },
        ];
        mockInvoke.mockResolvedValue(mockTree);

        const result = await api.getVirtualFileTree('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('get_virtual_file_tree', {
          workspaceId: 'ws-123',
        });
        expect(result).toEqual(mockTree);
      });

      it('应该处理获取文件树错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Tree unavailable'));

        await expect(api.getVirtualFileTree('ws-123')).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 状态同步测试
  // ========================================================================

  describe('状态同步', () => {
    describe('initStateSync', () => {
      it('应该成功初始化状态同步', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await api.initStateSync();

        expect(mockInvoke).toHaveBeenCalledWith('init_state_sync');
      });

      it('应该处理初始化错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Init failed'));

        await expect(api.initStateSync()).rejects.toThrow();
      });
    });

    describe('getWorkspaceState', () => {
      it('应该成功获取工作区状态', async () => {
        const mockState = {
          status: 'READY',
          lastUpdated: Date.now(),
        };
        mockInvoke.mockResolvedValue(mockState);

        const result = await api.getWorkspaceState('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('get_workspace_state', {
          workspaceId: 'ws-123',
        });
        expect(result).toEqual(mockState);
      });

      it('应该处理获取状态错误', async () => {
        mockInvoke.mockRejectedValue(new Error('State unavailable'));

        await expect(api.getWorkspaceState('ws-123')).rejects.toThrow();
      });
    });

    describe('getEventHistory', () => {
      it('应该成功获取事件历史', async () => {
        const mockEvents = [
          { id: '1', type: 'file_added', timestamp: Date.now() },
          { id: '2', type: 'file_modified', timestamp: Date.now() },
        ];
        mockInvoke.mockResolvedValue(mockEvents);

        const result = await api.getEventHistory({
          workspaceId: 'ws-123',
          limit: 100,
        });

        expect(mockInvoke).toHaveBeenCalledWith('get_event_history', {
          workspaceId: 'ws-123',
          limit: 100,
        });
        expect(result).toEqual(mockEvents);
      });

      it('应该支持不带 limit 的参数', async () => {
        mockInvoke.mockResolvedValue([]);

        await api.getEventHistory({ workspaceId: 'ws-123' });

        expect(mockInvoke).toHaveBeenCalledWith('get_event_history', {
          workspaceId: 'ws-123',
        });
      });

      it('应该处理获取历史错误', async () => {
        mockInvoke.mockRejectedValue(new Error('History unavailable'));

        await expect(
          api.getEventHistory({ workspaceId: 'ws-123' })
        ).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 缓存管理测试
  // ========================================================================

  describe('缓存管理', () => {
    describe('invalidateWorkspaceCache', () => {
      it('应该成功清理工作区缓存', async () => {
        mockInvoke.mockResolvedValue(5);

        const result = await api.invalidateWorkspaceCache('ws-123');

        expect(mockInvoke).toHaveBeenCalledWith('invalidate_workspace_cache', {
          workspaceId: 'ws-123',
        });
        expect(result).toBe(5);
      });

      it('应该处理清理缓存错误', async () => {
        mockInvoke.mockRejectedValue(new Error('Invalidation failed'));

        await expect(
          api.invalidateWorkspaceCache('ws-123')
        ).rejects.toThrow();
      });
    });
  });

  // ========================================================================
  // 错误处理集成测试
  // ========================================================================

  describe('错误处理集成', () => {
    it('所有方法都应该正确处理错误', async () => {
      // 测试所有 API 方法的错误处理
      const apiMethods = [
        () => api.loadWorkspace('ws-123'),
        () => api.refreshWorkspace('ws-123'),
        () => api.deleteWorkspace('ws-123'),
        () => api.getWorkspaceStatus('ws-123'),
        () => api.createWorkspace('Test', '/path'),
        () => api.searchLogs({ query: 'test' }),
        () => api.cancelSearch('search-123'),
        () => api.asyncSearchLogs({ query: 'test' }),
        () => api.cancelAsyncSearch('search-123'),
        () => api.importFolder('/path', 'ws-123'),
        () => api.checkRarSupport(),
        () => api.startWatch({ workspaceId: 'ws-123' }),
        () => api.stopWatch('ws-123'),
        () => api.cancelTask('task-123'),
        () => api.loadConfig(),
        () => api.getFileFilterConfig(),
        () => api.getPerformanceMetrics(),
        () => api.readFileByHash({ workspaceId: 'ws-123', hash: 'abc' }),
        () => api.getVirtualFileTree('ws-123'),
        () => api.initStateSync(),
        () => api.getWorkspaceState('ws-123'),
        () => api.getEventHistory({ workspaceId: 'ws-123' }),
        () => api.invalidateWorkspaceCache('ws-123'),
      ];

      // 为每个方法设置错误并验证
      for (const method of apiMethods) {
        mockInvoke.mockRejectedValueOnce(new Error('Test error'));
        await expect(method()).rejects.toThrow();
      }
    });

    it('应该正确传递命令名称到错误处理器', async () => {
      mockInvoke.mockRejectedValue(new Error('Test error'));

      try {
        await api.loadWorkspace('ws-123');
      } catch (error) {
        expect(error).toBeInstanceOf(Error);
      }
    });
  });

  // ========================================================================
  // 单例导出测试
  // ========================================================================

  describe('API 单例', () => {
    it('应该导出 api 对象', () => {
      expect(api).toBeDefined();
      expect(typeof api).toBe('object');
      // 验证关键方法存在
      expect(typeof api.loadWorkspace).toBe('function');
      expect(typeof api.searchLogs).toBe('function');
      expect(typeof api.importFolder).toBe('function');
    });

    it('多次导入应该返回相同的实例', async () => {
      // 重新导入验证单例
      const { api: api2 } = await import('../api');
      expect(api2).toBe(api);
    });
  });

  // ========================================================================
  // 参数验证测试
  // ========================================================================

  describe('参数传递', () => {
    it('应该正确传递所有搜索参数', async () => {
      mockInvoke.mockResolvedValue('search-id');

      const params: SearchParams = {
        query: 'complex query',
        workspaceId: 'ws-123',
        maxResults: 500,
        filters: {
          levels: ['ERROR', 'WARN', 'INFO'],
          timeRange: {
            start: '2024-01-01T00:00:00Z',
            end: '2024-12-31T23:59:59Z',
          },
          filePattern: '*.log',
        },
      };

      await api.searchLogs(params);

      expect(mockInvoke).toHaveBeenCalledWith('search_logs', params);
    });

    it('应该正确传递导出参数', async () => {
      mockInvoke.mockResolvedValue('/export/path.json');

      const params: ExportParams = {
        results: [
          { id: '1', content: 'line 1' },
          { id: '2', content: 'line 2' },
        ],
        format: 'json',
        savePath: '/export/path.json',
      };

      await api.exportResults(params);

      expect(mockInvoke).toHaveBeenCalledWith('export_results', params);
    });

    it('应该正确传递文件读取参数', async () => {
      mockInvoke.mockResolvedValue({ content: 'test' });

      const params = {
        workspaceId: 'ws-123',
        hash: 'def456',
        maxLength: 5000,
      };

      await api.readFileByHash(params);

      expect(mockInvoke).toHaveBeenCalledWith('read_file_by_hash', params);
    });
  });

  // ========================================================================
  // 返回类型测试
  // ========================================================================

  describe('返回类型', () => {
    it('loadWorkspace 应该返回正确的类型', async () => {
      const mockResponse = {
        id: 'ws-123',
        name: 'Test',
        path: '/path',
        status: 'READY' as const,
        fileCount: 10,
        totalSize: 100,
      };
      mockInvoke.mockResolvedValue(mockResponse);

      const result = await api.loadWorkspace('ws-123');

      expect(result.id).toBe('ws-123');
      expect(result.status).toBe('READY');
    });

    it('searchLogs 应该返回搜索 ID 字符串', async () => {
      mockInvoke.mockResolvedValue('search-abc123');

      const result = await api.searchLogs({ query: 'test' });

      expect(typeof result).toBe('string');
    });

    it('exportResults 应该返回文件路径字符串', async () => {
      mockInvoke.mockResolvedValue('/path/to/export.csv');

      const result = await api.exportResults({
        results: [],
        format: 'csv',
        savePath: '/path/to/export.csv',
      });

      expect(typeof result).toBe('string');
    });

    it('invalidateWorkspaceCache 应该返回数字', async () => {
      mockInvoke.mockResolvedValue(10);

      const result = await api.invalidateWorkspaceCache('ws-123');

      expect(typeof result).toBe('number');
    });
  });

  // ========================================================================
  // 边界条件测试
  // ========================================================================

  describe('边界条件', () => {
    it('应该处理空字符串参数', async () => {
      mockInvoke.mockResolvedValue('result');

      await api.searchLogs({ query: '', workspaceId: '' });

      expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
        query: '',
        workspaceId: '',
      });
    });

    it('应该处理空结果数组', async () => {
      mockInvoke.mockResolvedValue([]);

      const result = await api.getVirtualFileTree('ws-123');

      expect(result).toEqual([]);
    });

    it('应该处理 undefined 可选参数', async () => {
      mockInvoke.mockResolvedValue('search-id');

      await api.searchLogs({
        query: 'test',
        workspaceId: undefined,
        maxResults: undefined,
      });

      expect(mockInvoke).toHaveBeenCalledWith('search_logs', {
        query: 'test',
        workspaceId: undefined,
        maxResults: undefined,
      });
    });

    it('应该处理零值', async () => {
      mockInvoke.mockResolvedValue(0);

      const result = await api.invalidateWorkspaceCache('ws-123');

      expect(result).toBe(0);
    });
  });
});
