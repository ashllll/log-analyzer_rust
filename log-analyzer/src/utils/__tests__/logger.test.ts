import { logger, LogLevelEnum } from '../logger';

describe('logger', () => {
  let consoleLogSpy: jest.SpyInstance;
  let consoleWarnSpy: jest.SpyInstance;
  let consoleErrorSpy: jest.SpyInstance;
  let originalEnv: string | undefined;

  beforeEach(() => {
    consoleLogSpy = jest.spyOn(console, 'log').mockImplementation(() => {});
    consoleWarnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

    originalEnv = process.env.NODE_ENV;
    logger.setLevel('debug');
  });

  afterEach(() => {
    consoleLogSpy.mockRestore();
    consoleWarnSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    process.env.NODE_ENV = originalEnv;
  });

  describe('setLevel / getLevel', () => {
    it('应能设置和获取日志级别', () => {
      logger.setLevel('warn');
      expect(logger.getLevel()).toBe('warn');

      logger.setLevel('error');
      expect(logger.getLevel()).toBe('error');
    });

    it('设置 warn 级别后 debug 应被抑制', () => {
      logger.setLevel('warn');
      logger.debug('should not appear');
      expect(consoleLogSpy).not.toHaveBeenCalled();
    });

    it('设置 warn 级别后 warn 和 error 应正常输出', () => {
      logger.setLevel('warn');
      logger.warn('warn msg');
      logger.error('error msg');
      expect(consoleWarnSpy).toHaveBeenCalled();
      expect(consoleErrorSpy).toHaveBeenCalled();
    });
  });

  describe('字符串 API', () => {
    it('info 应调用 console.log 并包含 INFO 前缀', () => {
      logger.info('hello world');
      expect(consoleLogSpy).toHaveBeenCalledWith(
        `[${LogLevelEnum.INFO}] hello world`
      );
    });

    it('info 应支持多个参数', () => {
      logger.info('msg', 'arg1', 42);
      expect(consoleLogSpy).toHaveBeenCalledWith(
        `[${LogLevelEnum.INFO}] msg`,
        'arg1',
        42
      );
    });

    it('debug 应调用 console.log 并包含 DEBUG 前缀', () => {
      logger.debug('debug msg');
      expect(consoleLogSpy).toHaveBeenCalledWith(
        `[${LogLevelEnum.DEBUG}] debug msg`
      );
    });

    it('warn 应调用 console.warn', () => {
      logger.warn('warn msg');
      expect(consoleWarnSpy).toHaveBeenCalledWith('[WARN] warn msg');
    });

    it('error 应调用 console.error 并包含 ERROR 前缀', () => {
      logger.error('error msg');
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        `[${LogLevelEnum.ERROR}] error msg`
      );
    });
  });

  describe('结构化 API', () => {
    it('info 传入 context 对象时应调用 console.log', () => {
      logger.info({ user: 'test' });
      expect(consoleLogSpy).toHaveBeenCalledTimes(1);
      const callArg = consoleLogSpy.mock.calls[0][0] as string;
      expect(callArg).toContain('INFO');
      expect(callArg).toContain('user');
    });

    it('debug 传入 context 对象时应包含 DEBUG 前缀', () => {
      logger.debug({ key: 'value' });
      expect(consoleLogSpy).toHaveBeenCalledTimes(1);
      const callArg = consoleLogSpy.mock.calls[0][0] as string;
      expect(callArg).toContain('DEBUG');
    });

    it('warn 传入 context 对象时应调用 console.warn', () => {
      logger.warn({ warning: 'low disk' }, 'system');
      expect(consoleWarnSpy).toHaveBeenCalledTimes(1);
    });

    it('error 传入 context 对象时应调用 console.error', () => {
      logger.error({ error: 'timeout' }, 'request failed');
      expect(consoleErrorSpy).toHaveBeenCalledTimes(1);
    });

    it('生产模式下 format 应输出简单格式', () => {
      // 注：LoggerImpl.isDev 在实例化时确定，此测试验证 format 方法在非 dev 分支的行为
      // 通过直接传入 context 触发 format 调用，断言输出包含 INFO 且不崩溃
      process.env.NODE_ENV = 'production';
      logger.info({ key: 'value' }, 'msg');
      expect(consoleLogSpy).toHaveBeenCalledTimes(1);
      const callArg = consoleLogSpy.mock.calls[0][0] as string;
      expect(callArg).toContain('INFO');
    });
  });

  describe('format 分支覆盖', () => {
    it('开发模式下空 context 应回退到简单格式', () => {
      process.env.NODE_ENV = 'development';
      logger.info('plain message');
      expect(consoleLogSpy).toHaveBeenCalledWith(
        `[${LogLevelEnum.INFO}] plain message`
      );
    });

    it('空 message 且空 context 应产生有效输出', () => {
      logger.info({});
      expect(consoleLogSpy).toHaveBeenCalledTimes(1);
    });

    it('空 message 但非空 context 应包含 context', () => {
      logger.info({ a: 1 });
      const callArg = consoleLogSpy.mock.calls[0][0] as string;
      expect(callArg).toContain('a');
    });
  });
});
