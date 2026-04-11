/**
 * Optimized Inference API - High-performance inference operations
 * 
 * This API provides optimized versions of inference operations using:
 * - Model caching to avoid repeated loading
 * - Batch processing for better throughput
 * - Parallel processing with rayon
 * - Pipeline architecture for reduced latency
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, AnnotationBox } from './types';

/// Monitor information
export interface MonitorInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  is_primary: boolean;
}

/// Desktop capture configuration
export interface OptimizedDesktopCaptureConfig {
  model_path: string;
  confidence: number;
  device: string;
  monitor: number;
  fps_limit: number;
}

/// Desktop capture status
export interface DesktopCaptureStatus {
  active_sessions: string[];
  total_sessions: number;
}

/// Video metadata
export interface VideoMetadata {
  duration: number;
  fps: number;
  frames: number;
  width: number;
  height: number;
}

/// Inference session info
export interface InferenceSession {
  inference_id: string;
  status: 'running' | 'completed' | 'failed';
}

/**
 * Start optimized desktop capture inference
 */
export async function startOptimizedDesktopCapture(
  config: OptimizedDesktopCaptureConfig
): Promise<ApiResponse<{ session_id: string }>> {
  try {
    return await invoke<ApiResponse<{ session_id: string }>>('optimized_desktop_capture_start', { config });
  } catch (error) {
    console.error('[API] startOptimizedDesktopCapture error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop optimized desktop capture
 */
export async function stopOptimizedDesktopCapture(
  sessionId: string
): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('optimized_desktop_capture_stop', { sessionId });
  } catch (error) {
    console.error('[API] stopOptimizedDesktopCapture error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get available monitors
 */
export async function getOptimizedMonitors(): Promise<ApiResponse<MonitorInfo[]>> {
  try {
    return await invoke<ApiResponse<MonitorInfo[]>>('optimized_get_monitors');
  } catch (error) {
    console.error('[API] getOptimizedMonitors error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get desktop capture status
 */
export async function getOptimizedDesktopStatus(): Promise<ApiResponse<DesktopCaptureStatus>> {
  try {
    return await invoke<ApiResponse<DesktopCaptureStatus>>('optimized_desktop_capture_status');
  } catch (error) {
    console.error('[API] getOptimizedDesktopStatus error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Load video and get metadata
 */
export async function loadOptimizedVideo(
  videoPath: string
): Promise<ApiResponse<VideoMetadata>> {
  try {
    return await invoke<ApiResponse<VideoMetadata>>('optimized_video_load', { videoPath });
  } catch (error) {
    console.error('[API] loadOptimizedVideo error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Start optimized video inference
 */
export async function startOptimizedVideoInference(
  config: VideoInferenceConfig
): Promise<ApiResponse<InferenceSession>> {
  try {
    const result = await invoke<ApiResponse<string>>('optimized_video_inference_start', { config });
    if (result.success && result.data) {
      return {
        success: true,
        data: {
          inference_id: result.data,
          status: 'running' as const,
        },
      };
    }
    return { success: false, error: result.error };
  } catch (error) {
    console.error('[API] startOptimizedVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop optimized video inference
 */
export async function stopOptimizedVideoInference(
  sessionId?: string
): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('optimized_video_inference_stop', { sessionId });
  } catch (error) {
    console.error('[API] stopOptimizedVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}
