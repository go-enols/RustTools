/**
 * Common API - File dialogs and shared utilities
 */
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { DialogResult, ApiResponse } from './types';

/**
 * Open folder selection dialog
 */
export async function selectFolder(title: string, defaultPath?: string): Promise<DialogResult> {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title,
      defaultPath,
    });
    if (selected === null) {
      return { canceled: true };
    }
    return { canceled: false, path: selected as string };
  } catch (error) {
    console.error('[API] selectFolder error:', error);
    return { canceled: true };
  }
}


/**
 * Open file selection dialog
 */
export async function selectFile(title: string, filters?: { name: string; extensions: string[] }[]): Promise<DialogResult> {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      title,
      filters,
    });
    if (selected === null) {
      return { canceled: true };
    }
    return { canceled: false, path: selected as string };
  } catch (error) {
    console.error('[API] selectFile error:', error);
    return { canceled: true };
  }
}

/**
 * Open multiple file selection dialog
 */
export async function selectFiles(title: string, filters?: { name: string; extensions: string[] }[]): Promise<DialogResult> {
  try {
    const selected = await open({
      directory: false,
      multiple: true,
      title,
      filters,
    });
    if (selected === null) {
      return { canceled: true };
    }
    return { canceled: false, paths: selected as string[] };
  } catch (error) {
    console.error('[API] selectFiles error:', error);
    return { canceled: true };
  }
}

/**
 * Get application version
 */
export async function getAppVersion(): Promise<ApiResponse<{ version: string; build: string }>> {
  try {
    return await invoke<ApiResponse<{ version: string; build: string }>>('app_version');
  } catch (error) {
    console.error('[API] getAppVersion error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Check for application updates
 */
export async function checkForUpdates(): Promise<ApiResponse<{ available: boolean; version?: string }>> {
  try {
    return await invoke<ApiResponse<{ available: boolean; version?: string }>>('app_check_updates');
  } catch (error) {
    console.error('[API] checkForUpdates error:', error);
    return { success: false, error: String(error) };
  }
}
