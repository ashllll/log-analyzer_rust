import { queryKeys, configQuery } from '../queries';

const mockLoadConfig = jest.fn();

jest.mock('../api', () => ({
  api: {
    loadConfig: () => mockLoadConfig(),
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
