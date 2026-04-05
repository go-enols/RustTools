/**
 * File API - Common file operations
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse } from './types';

/**
 * File or directory info
 */
export interface FileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: string;
}

/**
 * Read text file content
 */
export async function readTextFile(filePath: string): Promise<ApiResponse<string>> {
  try {
    return await invoke<ApiResponse<string>>('read_text_file', { path: filePath });
  } catch (error) {
    console.error('[API] readTextFile error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Read binary file as base64
 */
export async function readBinaryFile(filePath: string): Promise<ApiResponse<string>> {
  try {
    return await invoke<ApiResponse<string>>('read_binary_file', { path: filePath });
  } catch (error) {
    console.error('[API] readBinaryFile error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Write text to file
 */
export async function writeTextFile(filePath: string, content: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('write_text_file', { path: filePath, content });
  } catch (error) {
    console.error('[API] writeTextFile error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Delete a file
 */
export async function deleteFile(filePath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('delete_file', { path: filePath });
  } catch (error) {
    console.error('[API] deleteFile error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Rename a file or directory
 */
export async function renamePath(oldPath: string, newPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('rename_path', { old_path: oldPath, new_path: newPath });
  } catch (error) {
    console.error('[API] renamePath error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Create a directory
 */
export async function createDirectory(dirPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('create_directory', { path: dirPath });
  } catch (error) {
    console.error('[API] createDirectory error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Delete a directory
 */
export async function deleteDirectory(dirPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('delete_directory', { path: dirPath });
  } catch (error) {
    console.error('[API] deleteDirectory error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * List directory contents
 */
export async function listDirectory(dirPath: string): Promise<ApiResponse<FileInfo[]>> {
  try {
    return await invoke<ApiResponse<FileInfo[]>>('list_directory', { path: dirPath });
  } catch (error) {
    console.error('[API] listDirectory error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Copy a file
 */
export async function copyFile(sourcePath: string, destPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('copy_file', { source: sourcePath, dest: destPath });
  } catch (error) {
    console.error('[API] copyFile error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Check if path exists
 */
export async function pathExists(filePath: string): Promise<ApiResponse<boolean>> {
  try {
    return await invoke<ApiResponse<boolean>>('path_exists', { path: filePath });
  } catch (error) {
    console.error('[API] pathExists error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * File change event from watcher
 */
export interface FileChangeEvent {
  path: string;
  kind: 'create' | 'modify' | 'remove';
  parent: string;
}

/**
 * Start watching a directory for file changes
 */
export async function startWatch(dirPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('start_watch', { path: dirPath });
  } catch (error) {
    console.error('[API] startWatch error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop watching a directory
 */
export async function stopWatch(dirPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('stop_watch', { path: dirPath });
  } catch (error) {
    console.error('[API] stopWatch error:', error);
    return { success: false, error: String(error) };
  }
}
