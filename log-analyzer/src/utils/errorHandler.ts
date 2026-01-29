// 错误处理工具类
import { logger } from './logger';

/**
 * 错误映射表:将后端错误映射为用户友好的提示
 */
const ERROR_MAP: Record<string, { message: string; suggestion: string }> = {
  'Path canonicalization failed': {
    message: '路径无效或不存在',
    suggestion: '检查路径是否正确'
  },
  'Failed to lock': {
    message: '资源正在使用中',
    suggestion: '稍后重试'
  },
  'unrar command not found': {
    message: 'RAR 支持异常',
    suggestion: 'RAR 解压组件异常，请重新安装或联系维护'
  },
  'Invalid Regex': {
    message: '搜索表达式语法错误',
    suggestion: '检查正则表达式格式'
  },
  'Disk space': {
    message: '磁盘空间不足',
    suggestion: '清理磁盘空间后重试'
  },
  'Path does not exist': {
    message: '路径不存在',
    suggestion: '选择有效的文件或目录'
  },
  'Workspace ID cannot be empty': {
    message: '工作区 ID 不能为空',
    suggestion: '请选择一个工作区'
  },
  'Search query cannot be empty': {
    message: '搜索查询不能为空',
    suggestion: '输入搜索关键词'
  },
};

/**
 * 错误处理器:统一处理应用错误
 */
export class ErrorHandler {
  /**
   * 处理错误,返回用户友好的错误消息
   * @param error - 错误对象或字符串
   * @returns 格式化的错误消息
   */
  static handle(error: any): string {
    const errorStr = String(error);
    logger.error('Error occurred:', errorStr);
    
    // 匹配错误模式
    for (const [pattern, info] of Object.entries(ERROR_MAP)) {
      if (errorStr.includes(pattern)) {
        return `${info.message} - ${info.suggestion}`;
      }
    }
    
    // 默认错误消息
    if (errorStr.length > 100) {
      return '操作失败,请查看控制台详情';
    }
    return errorStr;
  }
  
  /**
   * 判断错误是否可重试
   * @param error - 错误对象
   * @returns 是否可重试
   */
  static isRetryable(error: any): boolean {
    const errorStr = String(error);
    return errorStr.includes('Failed to lock') || 
           errorStr.includes('Resource busy') ||
           errorStr.includes('timeout');
  }
}
