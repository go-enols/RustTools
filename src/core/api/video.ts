/**
 * Video API - Video inference
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, VideoInferenceConfig, AnnotationBox } from './types';

/**
 * Load video for inference
 */
export async function loadVideo(videoPath: string): Promise<ApiResponse<{ duration: number; fps: number; frames: number }>> {
  try {
    return await invoke<ApiResponse<{ duration: number; fps: number; frames: number }>>('video_load', { videoPath });
  } catch (error) {
    console.error('[API] loadVideo error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Start video inference
 */
export async function startVideoInference(
  config: VideoInferenceConfig,
  _onFrame?: (frameIndex: number, annotations: AnnotationBox[]) => void,
  _onProgress?: (progress: number) => void
): Promise<ApiResponse<{ inference_id: string }>> {
  try {
    return await invoke<ApiResponse<{ inference_id: string }>>('video_inference_start', { config });
  } catch (error) {
    console.error('[API] startVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Stop video inference
 */
export async function stopVideoInference(): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('video_inference_stop');
  } catch (error) {
    console.error('[API] stopVideoInference error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Capture screenshot from video
 */
export async function captureScreenshot(videoPath: string, timestampMs: number): Promise<ApiResponse<{ screenshot_path: string }>> {
  try {
    return await invoke<ApiResponse<{ screenshot_path: string }>>('video_capture_screenshot', { videoPath, timestampMs });
  } catch (error) {
    console.error('[API] captureScreenshot error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Extract frames from video
 */
export async function extractFrames(videoPath: string, intervalMs: number): Promise<ApiResponse<{ frames: string[] }>> {
  try {
    return await invoke<ApiResponse<{ frames: string[] }>>('video_extract_frames', { videoPath, intervalMs });
  } catch (error) {
    console.error('[API] extractFrames error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get inference results/screenshots
 */
export async function getInferenceResults(inferenceId: string): Promise<ApiResponse<{ screenshots: string[]; annotations: AnnotationBox[][] }>> {
  try {
    return await invoke<ApiResponse<{ screenshots: string[]; annotations: AnnotationBox[][] }>>('video_inference_results', { inferenceId });
  } catch (error) {
    console.error('[API] getInferenceResults error:', error);
    return { success: false, error: String(error) };
  }
}
