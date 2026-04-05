/**
 * API - YOLO-Flow Backend API
 *
 * Re-exports all API modules for convenient importing.
 * Import from this file to access all backend functionality.
 *
 * @example
 * import { createProject, selectFolder } from '@/core/api';
 */

// Types (shared)
export type * from './types';

// Common (dialogs, file operations)
export { selectFolder, selectFile, selectFiles, getAppVersion, checkForUpdates } from './common';

// File operations
export {
  readTextFile,
  readBinaryFile,
  writeTextFile,
  deleteFile,
  renamePath,
  createDirectory,
  deleteDirectory,
  listDirectory,
  copyFile,
  pathExists,
} from './file';

// Project management
export { createProject, openProject, getRecentProjects, saveProject } from './project';

// Dataset management
export {
  loadDataset,
  getDatasetStats,
  importImages,
  exportDataset,
  validateDataset,
  batchRename,
  preprocessDataset,
} from './dataset';

// Annotation
export {
  loadAnnotationImage,
  saveAnnotations,
  navigateImage,
  autoLabel,
  addClass,
  deleteClass,
} from './annotation';

// Training
export {
  startTraining,
  stopTraining,
  pauseTraining,
  resumeTraining,
  getTrainingLogs,
  exportTrainingResults,
} from './training';

// Model
export {
  getTrainedModels,
  loadModel,
  deleteModel,
  convertModel,
  getBaseModels,
  exportModelFormat,
} from './model';

// Video inference
export {
  loadVideo,
  startVideoInference,
  stopVideoInference,
  captureScreenshot,
  extractFrames,
  getInferenceResults,
} from './video';

// Device
export { getDevices, getDeviceStats, setDefaultDevice } from './device';

// Settings
export { loadSettings, saveSettings } from './settings';
