import { logger } from '../utils/logger';
import { safeInvoke, isEmptyString } from './nullSafeApi';

/**
 * File content response from backend
 */
export interface FileContentResponse {
  content: string;
  hash: string;
  size: number;
}

/**
 * 空值安全的文件读取（增强版）
 */
export async function readFileByHash(
  workspaceId: string,
  hash: string
): Promise<FileContentResponse | null> {
  try {
    // 空值检查
    if (isEmptyString(workspaceId)) {
      logger.warn('readFileByHash: workspaceId 为空');
      return null;
    }
    if (isEmptyString(hash)) {
      logger.warn('readFileByHash: hash 为空');
      return null;
    }

    logger.debug('Reading file by hash:', { workspaceId, hash });

    const response = await safeInvoke<FileContentResponse | null>(
      'read_file_by_hash',
      { workspaceId, hash },
      {
        timeoutMs: 10000,
        fallback: null,
        onError: (err) => logger.error('读取文件失败', { error: err.message })
      }
    );

    if (response) {
      logger.debug('Successfully read file:', { hash, size: response.size });
    }

    return response;
  } catch (error) {
    logger.error('Failed to read file by hash:', error);
    throw new Error(`Failed to read file: ${error}`);
  }
}

/**
 * 文件 API
 */
export const fileApi = {
  readByHash: readFileByHash
};
