/**
 * 优化推理 API - 提供高性能推理接口
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { AnnotationBox, VideoInferenceConfig } from './types';

// ==================== 类型定义 ====================

/** 优化视频信息 */
export interface OptimizedVideoInfo {
  duration: number;
  fps: number;
  frames: number;
  width: number;
  height: number;
}

/** 优化桌面显示器信息 */
export interface OptimizedMonitorInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  is_primary: boolean;
}

/** 命令响应 */
export interface CommandResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/** 优化桌面捕获配置 */
export interface OptimizedDesktopConfig {
  model_path: string;
  confidence: number;
  monitor: number;
  fps_limit: number;
}

// ==================== 视频推理 API ====================

/**
 * 加载视频元数据（优化版）
 */
export async function optimizedLoadVideo(videoPath: string): Promise<OptimizedVideoInfo> {
  const response = await invoke<CommandResponse<OptimizedVideoInfo>>('optimized_video_load', {
    videoPath,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '加载视频失败');
  }
  
  return response.data;
}

/**
 * 启动优化视频推理
 */
export async function startOptimizedVideoInference(config: VideoInferenceConfig): Promise<string> {
  const response = await invoke<CommandResponse<string>>('optimized_video_inference_start', {
    config,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '启动推理失败');
  }
  
  return response.data;
}

/**
 * 监听优化视频推理帧
 */
export async function listenOptimizedVideoFrame(
  callback: (sessionId: string, frameIndex: number, boxes: AnnotationBox[]) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; frame: number; boxes: AnnotationBox[] }>(
    'optimized-video-frame',
    (event) => {
      callback(event.payload.session_id, event.payload.frame, event.payload.boxes);
    }
  );
}

/**
 * 监听优化视频推理完成
 */
export async function listenOptimizedVideoComplete(
  callback: (result: { sessionId: string; success: boolean; frames?: number; error?: string }) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; success: boolean; frames?: number; error?: string }>(
    'optimized-video-complete',
    (event) => {
      callback({
        sessionId: event.payload.session_id,
        success: event.payload.success,
        frames: event.payload.frames,
        error: event.payload.error,
      });
    }
  );
}

/**
 * 捕获视频截图（优化版）
 */
export async function optimizedCaptureScreenshot(
  videoPath: string,
  timestampMs: number
): Promise<string> {
  const response = await invoke<CommandResponse<string>>('optimized_video_screenshot', {
    videoPath,
    timestampMs,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '截图失败');
  }
  
  return response.data;
}

/**
 * 提取视频帧（优化版）
 */
export async function optimizedExtractFrames(
  videoPath: string,
  intervalMs: number
): Promise<string[]> {
  const response = await invoke<CommandResponse<string[]>>('optimized_video_extract_frames', {
    videoPath,
    intervalMs,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '提取帧失败');
  }
  
  return response.data;
}

// ==================== 桌面捕获 API ====================

/**
 * 获取显示器列表（优化版）
 */
export async function optimizedGetMonitors(): Promise<OptimizedMonitorInfo[]> {
  const response = await invoke<CommandResponse<OptimizedMonitorInfo[]>>('optimized_get_monitors');
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取显示器失败');
  }
  
  return response.data;
}

/**
 * 启动优化桌面捕获
 */
export async function startOptimizedDesktopCapture(
  config: OptimizedDesktopConfig
): Promise<string> {
  const response = await invoke<CommandResponse<string>>('optimized_desktop_capture_start', {
    config,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '启动捕获失败');
  }
  
  return response.data;
}

/**
 * 停止优化桌面捕获
 */
export async function stopOptimizedDesktopCapture(sessionId: string): Promise<void> {
  const response = await invoke<CommandResponse<void>>('optimized_desktop_capture_stop', {
    sessionId,
  });
  
  if (!response.success) {
    throw new Error(response.error || '停止捕获失败');
  }
}

/**
 * 获取活动会话（优化版）
 */
export async function optimizedGetActiveSessions(): Promise<string[]> {
  const response = await invoke<CommandResponse<string[]>>('optimized_desktop_get_active_sessions');
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取会话失败');
  }
  
  return response.data;
}

/**
 * 监听优化桌面捕获帧
 */
export async function listenOptimizedDesktopFrame(
  callback: (frame: {
    session_id: string;
    image: string;
    boxes: AnnotationBox[];
    width: number;
    height: number;
    fps: number;
    timestamp: number;
  }) => void
): Promise<UnlistenFn> {
  return listen<{
    session_id: string;
    image: string;
    boxes: AnnotationBox[];
    width: number;
    height: number;
    fps: number;
    timestamp: number;
  }>('desktop-capture-frame', (event) => {
    callback(event.payload);
  });
}
