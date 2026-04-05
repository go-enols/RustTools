/**
 * Project API - Project management
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, ProjectConfig } from './types';

/**
 * Create a new YOLO project
 */
export async function createProject(config: ProjectConfig): Promise<ApiResponse<ProjectConfig>> {
  try {
    return await invoke<ApiResponse<ProjectConfig>>('project_create', { config });
  } catch (error) {
    console.error('[API] createProject error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Open an existing project
 */
export async function openProject(projectPath: string): Promise<ApiResponse<ProjectConfig>> {
  try {
    return await invoke<ApiResponse<ProjectConfig>>('project_open', { projectPath });
  } catch (error) {
    console.error('[API] openProject error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get recent projects list
 */
export async function getRecentProjects(): Promise<ApiResponse<string[]>> {
  try {
    return await invoke<ApiResponse<string[]>>('project_recent_list');
  } catch (error) {
    console.error('[API] getRecentProjects error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Save current project state
 */
export async function saveProject(): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('project_save');
  } catch (error) {
    console.error('[API] saveProject error:', error);
    return { success: false, error: String(error) };
  }
}
