/**
 * Video API - Video inference operations
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse } from './types';
import type { VideoInferenceConfig } from './types';

export interface VideoMetadata {
  duration: number;
  fps: number;
  frames: number;
  width: number;
  height: number;
}

export interface InferenceSession {
  inference_id: string;
  status: 'running' | 'completed' | 'failed';
}

/**
 * Load video and get metadata
 */
export async function loadVideo(videoPath: string): Promise<ApiResponse<VideoMetadata>> {
  try {
    return await invoke('video_load', { videoPath });
  } catch (error) {
    console.error('[API] loadVideo error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Start video inference (Python backend)
 */
export async function startVideoInference(
  config: VideoInferenceConfig
): Promise<ApiResponse<InferenceSession>> {
  try {
    const result = await invoke<ApiResponse<string>>('video_inference_start', { config });
    if (result.success && result.data) {
      return {
        success: true,
        data: {
          inference_id: result.data,
          status: 'running',
        },
      };
    }
    return { success: false, error: result.error };
  } catch (error) {
    console.error('[API] startVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Start video inference using pure Rust (NEW - optimized)
 */
export async function startRustVideoInference(
  config: VideoInferenceConfig
): Promise<ApiResponse<InferenceSession>> {
  try {
    const result = await invoke<ApiResponse<string>>('rust_video_inference_start', { config });
    if (result.success && result.data) {
      return {
        success: true,
        data: {
          inference_id: result.data,
          status: 'running',
        },
      };
    }
    return { success: false, error: result.error };
  } catch (error) {
    console.error('[API] startRustVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop video inference
 */
export async function stopVideoInference(sessionId?: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('video_inference_stop', { sessionId });
  } catch (error) {
    console.error('[API] stopVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop Rust video inference
 */
export async function stopRustVideoInference(sessionId?: string): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('rust_video_inference_stop', { sessionId });
  } catch (error) {
    console.error('[API] stopRustVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Capture screenshot from video
 */
export async function captureScreenshot(
  videoPath: string,
  timestampMs: number
): Promise<ApiResponse<string>> {
  try {
    return await invoke<ApiResponse<string>>('video_capture_screenshot', {
      videoPath,
      timestampMs,
    });
  } catch (error) {
    console.error('[API] captureScreenshot error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Extract frames from video
 */
export async function extractFrames(
  videoPath: string,
  intervalMs: number
): Promise<ApiResponse<string[]>> {
  try {
    return await invoke<ApiResponse<string[]>>('video_extract_frames', {
      videoPath,
      intervalMs,
    });
  } catch (error) {
    console.error('[API] extractFrames error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get inference results
 */
export async function getVideoInferenceResults(
  inferenceId: string
): Promise<ApiResponse<unknown>> {
  try {
    return await invoke<ApiResponse<unknown>>('video_inference_results', {
      inferenceId,
    });
  } catch (error) {
    console.error('[API] getVideoInferenceResults error:', error);
    return { success: false, error: String(error) };
  }
}
