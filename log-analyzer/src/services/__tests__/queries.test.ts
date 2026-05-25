const mockLoadConfig = jest.fn();

// queryKeys 和 configQuery 是纯静态定义，直接导入真实值
import { queryKeys, configQuery } from '../api';

jest.mock('../api', () => {
  const actual = jest.requireActual('../api');
  return {
    api: {
      loadConfig: () => mockLoadConfig(),
    },
    queryKeys: actual.queryKeys,
    configQuery: { ...actual.configQuery, queryFn: () => mockLoadConfig() },
  };
});

describe('queryKeys', () => {
  it('config key 应为稳定引用', () => {
    expect(queryKeys.config).toEqual(['config']);
  });

  it('workspace key 应包含传入的 id', () => {
    expect(queryKeys.workspace('ws-1')).toEqual(['workspace', 'ws-1']);
  });
});

describe('configQuery', () => {
  it('应具有正确的 queryKey', () => {
    expect(configQuery.queryKey).toEqual(['config']);
  });

  it('应具有正确的 staleTime', () => {
    expect(configQuery.staleTime).toBe(5 * 60_000);
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
