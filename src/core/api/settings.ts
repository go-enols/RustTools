/**
 * Settings API - Application settings
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse } from './types';

/**
 * Load application settings
 */
export async function loadSettings(): Promise<ApiResponse<Record<string, unknown>>> {
  try {
    return await invoke<ApiResponse<Record<string, unknown>>>('settings_load');
  } catch (error) {
    console.error('[API] loadSettings error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Save application settings
 */
export async function saveSettings(settings: Record<string, unknown>): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('settings_save', { settings });
  } catch (error) {
    console.error('[API] saveSettings error:', error);
    return { success: false, error: String(error) };
  }
}
