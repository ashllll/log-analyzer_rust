import { queryKeys, configQuery, virtualFileTreeQuery } from '../queries';

const mockLoadConfig = jest.fn();
const mockGetVirtualFileTree = jest.fn();

jest.mock('../api', () => ({
  api: {
    loadConfig: () => mockLoadConfig(),
    getVirtualFileTree: (id: string) => mockGetVirtualFileTree(id),
  },
}));

describe('queryKeys', () => {
  it('config key 应为稳定引用', () => {
    expect(queryKeys.config).toEqual(['config']);
  });

  it('workspaces key 应为稳定引用', () => {
    expect(queryKeys.workspaces).toEqual(['workspaces']);
  });

  it('workspace key 应包含传入的 id', () => {
    expect(queryKeys.workspace('ws-1')).toEqual(['workspace', 'ws-1']);
  });

  it('keywordGroups key 应为稳定引用', () => {
    expect(queryKeys.keywordGroups).toEqual(['keywordGroups']);
  });

  it('tasks key 应为稳定引用', () => {
    expect(queryKeys.tasks).toEqual(['tasks']);
  });

  it('virtualFileTree key 应包含传入的 workspaceId', () => {
    expect(queryKeys.virtualFileTree('ws-2')).toEqual([
      'virtualFileTree',
      'ws-2',
    ]);
  });

  it('cacheConfig key 应为稳定引用', () => {
    expect(queryKeys.cacheConfig).toEqual(['cacheConfig']);
  });

  it('searchConfig key 应为稳定引用', () => {
    expect(queryKeys.searchConfig).toEqual(['searchConfig']);
  });

  it('taskManagerConfig key 应为稳定引用', () => {
    expect(queryKeys.taskManagerConfig).toEqual(['taskManagerConfig']);
  });
});

describe('configQuery', () => {
  it('应具有正确的 queryKey', () => {
    expect(configQuery.queryKey).toEqual(['config']);
  });

  it('应具有正确的 staleTime', () => {
    expect(configQuery.staleTime).toBe(60_000);
  });

  it('应具有正确的 gcTime', () => {
    expect(configQuery.gcTime).toBe(300_000);
  });

  it('queryFn 应调用 api.loadConfig', async () => {
    const mockConfig = { theme: 'dark' };
    mockLoadConfig.mockResolvedValue(mockConfig);
    const result = await configQuery.queryFn();
    expect(mockLoadConfig).toHaveBeenCalledTimes(1);
    expect(result).toEqual(mockConfig);
  });
});

describe('virtualFileTreeQuery', () => {
  it('非空 workspaceId 时应启用查询', () => {
    const query = virtualFileTreeQuery('ws-1');
    expect(query.enabled).toBe(true);
    expect(query.queryKey).toEqual(['virtualFileTree', 'ws-1']);
  });

  it('空 workspaceId 时应禁用查询', () => {
    const query = virtualFileTreeQuery('');
    expect(query.enabled).toBe(false);
  });

  it('应具有正确的 staleTime 和 gcTime', () => {
    const query = virtualFileTreeQuery('ws-1');
    expect(query.staleTime).toBe(30_000);
    expect(query.gcTime).toBe(300_000);
  });

  it('queryFn 应调用 api.getVirtualFileTree 并传入 workspaceId', async () => {
    const mockTree = [{ id: 'node-1', name: 'root' }];
    mockGetVirtualFileTree.mockResolvedValue(mockTree);
    const query = virtualFileTreeQuery('ws-1');
    const result = await query.queryFn();
    expect(mockGetVirtualFileTree).toHaveBeenCalledWith('ws-1');
    expect(result).toEqual(mockTree);
  });
});
