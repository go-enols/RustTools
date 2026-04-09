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
export async function stopTraining(trainingId: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_stop', { trainingId });
  } catch (error) {
    console.error('[API] stopTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Pause current training
 */
export async function pauseTraining(trainingId: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_pause', { trainingId });
  } catch (error) {
    console.error('[API] pauseTraining error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Resume paused training
 */
export async function resumeTraining(trainingId: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('training_resume', { trainingId });
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

export interface ModelCheckResult {
  exists: boolean;
  model: string;
  path: string | null;
}

export interface ModelDownloadResult {
  success: boolean;
  model: string;
  path: string | null;
  error: string | null;
}

// Python Environment Types
export interface PythonEnvInfo {
  python_exists: boolean;
  python_version: string | null;
  torch_exists: boolean;
  torch_version: string | null;
  ultralytics_exists: boolean;
  ultralytics_version: string | null;
  yolo_command_exists: boolean;
}

export interface InstallInstructions {
  pip_install: string[];
  torch_install: string[];
  torch_cpu_install: string[];
  ultralytics_install: string[];
  manual_download: string[];
}

/**
 * Check Python environment (Python, PyTorch, Ultralytics)
 */
export async function checkPythonEnv(): Promise<ApiResponse<PythonEnvInfo>> {
  try {
    return await invoke<ApiResponse<PythonEnvInfo>>('check_python_env');
  } catch (error) {
    console.error('[API] checkPythonEnv error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Install Python dependencies (torch, ultralytics)
 */
export async function installPythonDeps(useMirror: boolean = true): Promise<ApiResponse<{ success: boolean; message: string }>> {
  try {
    return await invoke<ApiResponse<{ success: boolean; message: string }>>('install_python_deps', { useMirror });
  } catch (error) {
    console.error('[API] installPythonDeps error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get manual installation instructions
 */
export async function getInstallInstructions(): Promise<InstallInstructions> {
  try {
    return await invoke<InstallInstructions>('get_install_instructions');
  } catch (error) {
    console.error('[API] getInstallInstructions error:', error);
    return {
      pip_install: ['python -m pip install --upgrade pip'],
      torch_install: ['pip install torch', 'pip install torch -i https://pypi.tuna.tsinghua.edu.cn/simple'],
      ultralytics_install: ['pip install ultralytics', 'pip install ultralytics -i https://pypi.tuna.tsinghua.edu.cn/simple'],
      manual_download: ['https://pytorch.org/get-started/locally/', 'https://docs.ultralytics.com/'],
    };
  }
}

/**
 * Check if a YOLO model exists locally
 */
export async function checkModel(modelName: string): Promise<ApiResponse<ModelCheckResult>> {
  try {
    return await invoke<ApiResponse<ModelCheckResult>>('yolo_check_model', { modelName });
  } catch (error) {
    console.error('[API] checkModel error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Download a YOLO model
 */
export async function downloadModel(modelName: string): Promise<ApiResponse<ModelDownloadResult>> {
  try {
    return await invoke<ApiResponse<ModelDownloadResult>>('yolo_download_model', { modelName });
  } catch (error) {
    console.error('[API] downloadModel error:', error);
    return { success: false, error: String(error) };
  }
}
