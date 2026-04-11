/**
 * Desktop Capture API - Real-time desktop capture inference
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, AnnotationBox } from './types';

export interface DesktopCaptureConfig {
  model_path: string;
  confidence: number;
  device: string;
  monitor: number;
  fps_limit: number;
}

export interface DesktopDetection {
  session_id: string;
  boxes: AnnotationBox[];
  width: number;
  height: number;
}

/**
 * Start desktop capture inference
 */
export async function startDesktopCapture(
  config: DesktopCaptureConfig
): Promise<ApiResponse<{ session_id: string }>> {
  try {
    return await invoke<ApiResponse<{ session_id: string }>>('desktop_capture_start', { config });
  } catch (error) {
    console.error('[API] startDesktopCapture error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop desktop capture inference
 */
export async function stopDesktopCapture(
  sessionId: string
): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('desktop_capture_stop', { sessionId });
  } catch (error) {
    console.error('[API] stopDesktopCapture error:', error);
    return { success: false, error: String(error) };
  }
}
