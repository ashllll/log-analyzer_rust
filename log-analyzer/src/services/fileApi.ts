import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';

/**
 * File content response from backend
 */
export interface FileContentResponse {
  content: string;
  hash: string;
  size: number;
}

/**
 * Read file content by SHA-256 hash
 * 
 * This function retrieves file content from the Content-Addressable Storage
 * using the file's SHA-256 hash.
 * 
 * @param workspaceId - ID of the workspace containing the file
 * @param hash - SHA-256 hash of the file
 * @returns File content and metadata
 * 
 * @example
 * ```typescript
 * const content = await readFileByHash('workspace_123', 'a3f2e1d4...');
 * console.log(content.content);
 * ```
 */
export async function readFileByHash(
  workspaceId: string,
  hash: string
): Promise<FileContentResponse> {
  try {
    logger.debug('Reading file by hash:', { workspaceId, hash });
    
    const response = await invoke<FileContentResponse>('read_file_by_hash', {
      workspaceId,
      hash
    });
    
    logger.debug('Successfully read file:', { hash, size: response.size });
    return response;
  } catch (error) {
    logger.error('Failed to read file by hash:', error);
    throw new Error(`Failed to read file: ${error}`);
  }
}

/**
 * File API
 */
export const fileApi = {
  readByHash: readFileByHash
};
