/**
 * Model API - Model management
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, TrainedModel, ConvertConfig, ConvertProgress } from './types';

/**
 * Get list of trained models
 */
export async function getTrainedModels(): Promise<ApiResponse<TrainedModel[]>> {
  try {
    return await invoke<ApiResponse<TrainedModel[]>>('model_list');
  } catch (error) {
    console.error('[API] getTrainedModels error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Load trained model
 */
export async function loadModel(modelPath: string): Promise<ApiResponse<{ model_id: string }>> {
  try {
    return await invoke<ApiResponse<{ model_id: string }>>('model_load', { modelPath });
  } catch (error) {
    console.error('[API] loadModel error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Delete trained model
 */
export async function deleteModel(modelId: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('model_delete', { modelId });
  } catch (error) {
    console.error('[API] deleteModel error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Convert model to edge format
 */
export async function convertModel(
  config: ConvertConfig,
  _onProgress?: (progress: ConvertProgress) => void
): Promise<ApiResponse<{ output_path: string }>> {
  try {
    return await invoke<ApiResponse<{ output_path: string }>>('model_convert', { config });
  } catch (error) {
    console.error('[API] convertModel error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get available base models
 */
export async function getBaseModels(): Promise<ApiResponse<{ name: string; path: string; size: string }[]>> {
  try {
    return await invoke<ApiResponse<{ name: string; path: string; size: string }[]>>('model_base_list');
  } catch (error) {
    console.error('[API] getBaseModels error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Export model to different format
 */
export async function exportModelFormat(
  modelPath: string,
  format: 'onnx' | 'torchscript' | 'coreml' | 'tflite'
): Promise<ApiResponse<{ output_path: string }>> {
  try {
    return await invoke<ApiResponse<{ output_path: string }>>('tools_export', { modelPath, format });
  } catch (error) {
    console.error('[API] exportModelFormat error:', error);
    return { success: false, error: String(error) };
  }
}
