/**
 * Dataset API - Dataset management
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, DatasetStats } from './types';

/**
 * Load dataset from folder
 */
export async function loadDataset(folderPath: string): Promise<ApiResponse<DatasetStats>> {
  try {
    return await invoke<ApiResponse<DatasetStats>>('dataset_load', { folderPath });
  } catch (error) {
    console.error('[API] loadDataset error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get dataset statistics
 */
export async function getDatasetStats(): Promise<ApiResponse<DatasetStats>> {
  try {
    return await invoke<ApiResponse<DatasetStats>>('dataset_stats');
  } catch (error) {
    console.error('[API] getDatasetStats error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Import images to dataset
 */
export async function importImages(sourcePaths: string[], targetFolder: string): Promise<ApiResponse<{ imported: number }>> {
  try {
    return await invoke<ApiResponse<{ imported: number }>>('dataset_import_images', { sourcePaths, targetFolder });
  } catch (error) {
    console.error('[API] importImages error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Export dataset in YOLO format
 */
export async function exportDataset(outputPath: string): Promise<ApiResponse<{ exported: number }>> {
  try {
    return await invoke<ApiResponse<{ exported: number }>>('dataset_export', { outputPath });
  } catch (error) {
    console.error('[API] exportDataset error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Validate dataset structure
 */
export async function validateDataset(datasetPath: string): Promise<ApiResponse<{ valid: boolean; errors: string[] }>> {
  try {
    return await invoke<ApiResponse<{ valid: boolean; errors: string[] }>>('tools_validate_dataset', { datasetPath });
  } catch (error) {
    console.error('[API] validateDataset error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Batch rename dataset files
 */
export async function batchRename(folderPath: string, pattern: string): Promise<ApiResponse<{ renamed: number }>> {
  try {
    return await invoke<ApiResponse<{ renamed: number }>>('tools_batch_rename', { folderPath, pattern });
  } catch (error) {
    console.error('[API] batchRename error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Run dataset preprocessing
 */
export async function preprocessDataset(
  datasetPath: string,
  options: { resize?: number; normalize?: boolean; augment?: boolean }
): Promise<ApiResponse<{ processed: number }>> {
  try {
    return await invoke<ApiResponse<{ processed: number }>>('tools_preprocess', { datasetPath, options });
  } catch (error) {
    console.error('[API] preprocessDataset error:', error);
    return { success: false, error: String(error) };
  }
}
