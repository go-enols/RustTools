/**
 * Training API - Model training
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, TrainingConfig, TrainingProgress } from './types';

/**
 * Start YOLO training
 */
export async function startTraining(
  projectPath: string,
  config: TrainingConfig,
  _onProgress?: (progress: TrainingProgress) => void
): Promise<ApiResponse<{ training_id: string }>> {
  try {
    return await invoke<ApiResponse<{ training_id: string }>>('training_start', { projectPath, config });
  } catch (error) {
    console.error('[API] startTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop current training
 */
export async function stopTraining(): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_stop');
  } catch (error) {
    console.error('[API] stopTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Pause current training
 */
export async function pauseTraining(): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_pause');
  } catch (error) {
    console.error('[API] pauseTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Resume paused training
 */
export async function resumeTraining(): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_resume');
  } catch (error) {
    console.error('[API] resumeTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get training history/logs
 */
export async function getTrainingLogs(trainingId: string): Promise<ApiResponse<string[]>> {
  try {
    return await invoke<ApiResponse<string[]>>('training_logs', { trainingId });
  } catch (error) {
    console.error('[API] getTrainingLogs error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Export training results
 */
export async function exportTrainingResults(trainingId: string, outputPath: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_export', { trainingId, outputPath });
  } catch (error) {
    console.error('[API] exportTrainingResults error:', error);
    return { success: false, error: String(error) };
  }
}
