/**
 * 优化推理 API v2 - 高性能推理接口
 * 
 * 完全纯 Rust 实现，无 Python 依赖
 * 
 * 特性：
 * - 模型缓存复用
 * - SIMD 优化预处理
 * - 自适应帧率控制
 * - 流式视频处理
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

/** 模型兼容性信息 */
export interface ModelCompatibility {
  is_compatible: boolean;
  format: string;
  message: string;
  conversion_hint?: string;
}

/** 性能统计 */
export interface PerformanceStats {
  fps: number;
  latency_ms: number;
  memory_mb: number;
  cpu_percent: number;
  frames_processed: number;
}

/** 推理会话状态 */
export interface InferenceSessionState {
  session_id: string;
  is_running: boolean;
  frames_processed: number;
  fps_achieved: number;
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
 * 启动优化视频推理（纯 Rust 实现）
 */
export async function startOptimizedVideoInference(config: VideoInferenceConfig): Promise<string> {
  const response = await invoke<CommandResponse<string>>('rust_video_inference_start', {
    config,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '启动推理失败');
  }
  
  return response.data;
}

/**
 * 停止视频推理
 */
export async function stopOptimizedVideoInference(sessionId?: string): Promise<void> {
  const response = await invoke<CommandResponse<void>>('rust_video_inference_stop', {
    sessionId,
  });
  
  if (!response.success) {
    throw new Error(response.error || '停止推理失败');
  }
}

/**
 * 监听优化视频推理帧（实时回调）
 */
export async function listenOptimizedVideoFrame(
  callback: (sessionId: string, frameIndex: number, boxes: AnnotationBox[]) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; frame: number; boxes: AnnotationBox[] }>(
    'rust-video-inference-frame',
    (event) => {
      callback(event.payload.session_id, event.payload.frame, event.payload.boxes);
    }
  );
}

/**
 * 监听优化视频推理完成
 */
export async function listenOptimizedVideoComplete(
  callback: (result: {
    sessionId: string;
    success: boolean;
    frames?: number;
    error?: string;
  }) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; success: boolean; frames?: number; error?: string }>(
    'rust-video-inference-complete',
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
  const response = await invoke<CommandResponse<OptimizedMonitorInfo[]>>('get_monitors');
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取显示器失败');
  }
  
  return response.data;
}

/**
 * 启动优化桌面捕获（纯 Rust 实现）
 */
export async function startOptimizedDesktopCapture(
  config: OptimizedDesktopConfig
): Promise<string> {
  const response = await invoke<CommandResponse<string>>('desktop_capture_start', {
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
  const response = await invoke<CommandResponse<void>>('desktop_capture_stop', {
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

// ==================== 模型管理 API ====================

/**
 * 检测模型格式
 */
export async function detectModelFormat(modelPath: string): Promise<string> {
  const response = await invoke<CommandResponse<string>>('detect_model_format_cmd', {
    path: modelPath,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '检测模型格式失败');
  }
  
  return response.data;
}

/**
 * 获取模型信息
 */
export async function getModelInfo(modelPath: string): Promise<string> {
  const response = await invoke<CommandResponse<string>>('get_model_info_cmd', {
    path: modelPath,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取模型信息失败');
  }
  
  return response.data;
}

/**
 * 检查模型兼容性
 */
export async function checkModelCompatibility(modelPath: string): Promise<ModelCompatibility> {
  const response = await invoke<CommandResponse<ModelCompatibility>>('check_model_compatibility', {
    path: modelPath,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '检查模型兼容性失败');
  }
  
  return response.data;
}

/**
 * 获取模型转换说明
 */
export async function getConversionInstructions(modelPath: string): Promise<string> {
  const response = await invoke<CommandResponse<string>>('get_conversion_instructions_cmd', {
    path: modelPath,
  });
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取转换说明失败');
  }
  
  return response.data;
}

// ==================== 性能监控 API ====================

/**
 * 获取推理会话状态
 */
export async function getInferenceSessionState(sessionId: string): Promise<InferenceSessionState> {
  const response = await invoke<CommandResponse<InferenceSessionState>>(
    'rust_video_inference_get_state',
    { sessionId }
  );
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取会话状态失败');
  }
  
  return response.data;
}

/**
 * 获取桌面捕获服务状态
 */
export async function getDesktopCaptureStatus(): Promise<{
  active_sessions: string[];
  total_sessions: number;
}> {
  const response = await invoke<CommandResponse<{
    active_sessions: string[];
    total_sessions: number;
  }>>('get_desktop_capture_status');
  
  if (!response.success || !response.data) {
    throw new Error(response.error || '获取捕获状态失败');
  }
  
  return response.data;
}

// ==================== 工具函数 ====================

/**
 * 性能监控器类
 */
export class PerformanceMonitor {
  private frameCount: number = 0;
  private startTime: number = Date.now();
  private lastFrameTime: number = Date.now();
  private frameTimes: number[] = [];
  private maxSamples: number = 100;
  
  /**
   * 记录帧
   */
  recordFrame(): void {
    const now = Date.now();
    const frameTime = now - this.lastFrameTime;
    
    this.frameTimes.push(frameTime);
    if (this.frameTimes.length > this.maxSamples) {
      this.frameTimes.shift();
    }
    
    this.frameCount++;
    this.lastFrameTime = now;
  }
  
  /**
   * 获取当前 FPS
   */
  getFPS(): number {
    if (this.frameTimes.length === 0) return 0;
    
    const avgFrameTime = this.frameTimes.reduce((a, b) => a + b, 0) / this.frameTimes.length;
    return avgFrameTime > 0 ? 1000 / avgFrameTime : 0;
  }
  
  /**
   * 获取平均延迟 (ms)
   */
  getAverageLatency(): number {
    if (this.frameTimes.length === 0) return 0;
    return this.frameTimes.reduce((a, b) => a + b, 0) / this.frameTimes.length;
  }
  
  /**
   * 获取总体 FPS
   */
  getOverallFPS(): number {
    const elapsed = Date.now() - this.startTime;
    return elapsed > 0 ? (this.frameCount / elapsed) * 1000 : 0;
  }
  
  /**
   * 获取统计信息
   */
  getStats(): PerformanceStats {
    return {
      fps: this.getFPS(),
      latency_ms: this.getAverageLatency(),
      memory_mb: 0, // 需要原生实现
      cpu_percent: 0, // 需要原生实现
      frames_processed: this.frameCount,
    };
  }
  
  /**
   * 重置监控器
   */
  reset(): void {
    this.frameCount = 0;
    this.startTime = Date.now();
    this.lastFrameTime = Date.now();
    this.frameTimes = [];
  }
}

/**
 * 检测结果可视化
 */
export class DetectionVisualizer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private colors: string[] = [
    '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4',
    '#FFEAA7', '#DDA0DD', '#FF9F43', '#C8D6E5',
  ];
  
  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d')!;
  }
  
  /**
   * 绘制检测框
   */
  drawBoxes(image: string, boxes: AnnotationBox[]): void {
    const img = new Image();
    img.onload = () => {
      // 设置 Canvas 尺寸
      this.canvas.width = img.width;
      this.canvas.height = img.height;
      
      // 绘制图像
      this.ctx.drawImage(img, 0, 0);
      
      // 绘制检测框
      boxes.forEach((box, index) => {
        const color = this.colors[box.class_id % this.colors.length];
        
        // 绘制边框
        this.ctx.strokeStyle = color;
        this.ctx.lineWidth = 3;
        this.ctx.strokeRect(box.x, box.y, box.width, box.height);
        
        // 绘制标签背景
        this.ctx.fillStyle = color;
        const label = `${box.class_name} ${(box.confidence * 100).toFixed(1)}%`;
        const textMetrics = this.ctx.measureText(label);
        this.ctx.fillRect(box.x, box.y - 25, textMetrics.width + 10, 25);
        
        // 绘制标签文字
        this.ctx.fillStyle = '#FFFFFF';
        this.ctx.font = 'bold 16px Arial';
        this.ctx.fillText(label, box.x + 5, box.y - 5);
      });
    };
    img.src = `data:image/jpeg;base64,${image}`;
  }
  
  /**
   * 绘制带透明度的检测框
   */
  drawBoxesWithAlpha(image: string, boxes: AnnotationBox[], alpha: number = 0.3): void {
    const img = new Image();
    img.onload = () => {
      this.canvas.width = img.width;
      this.canvas.height = img.height;
      this.ctx.drawImage(img, 0, 0);
      
      boxes.forEach((box) => {
        const color = this.colors[box.class_id % this.colors.length];
        
        // 绘制透明填充
        this.ctx.fillStyle = color + Math.floor(alpha * 255).toString(16).padStart(2, '0');
        this.ctx.fillRect(box.x, box.y, box.width, box.height);
        
        // 绘制边框
        this.ctx.strokeStyle = color;
        this.ctx.lineWidth = 2;
        this.ctx.strokeRect(box.x, box.y, box.width, box.height);
      });
    };
    img.src = `data:image/jpeg;base64,${image}`;
  }
  
  /**
   * 清除画布
   */
  clear(): void {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }
}

// ==================== 使用示例 ====================

/**
 * 示例: 实时桌面监控
 */
export async function exampleDesktopCapture() {
  const monitor = new PerformanceMonitor();
  
  // 1. 获取显示器
  const monitors = await optimizedGetMonitors();
  console.log('Available monitors:', monitors);
  
  // 2. 启动捕获
  const sessionId = await startOptimizedDesktopCapture({
    model_path: 'yolov8n.onnx',
    confidence: 0.5,
    monitor: 1,
    fps_limit: 30,
  });
  
  // 3. 监听帧
  const unlisten = await listenOptimizedDesktopFrame((frame) => {
    monitor.recordFrame();
    
    console.log(`FPS: ${monitor.getFPS().toFixed(2)}`);
    console.log(`Detections: ${frame.boxes.length}`);
    
    // 绘制到 Canvas
    const canvas = document.getElementById('canvas') as HTMLCanvasElement;
    const visualizer = new DetectionVisualizer(canvas);
    visualizer.drawBoxes(frame.image, frame.boxes);
  });
  
  // 4. 10秒后停止
  setTimeout(async () => {
    await stopOptimizedDesktopCapture(sessionId);
    unlisten();
    
    console.log('Final stats:', monitor.getStats());
  }, 10000);
}

/**
 * 示例: 视频批量推理
 */
export async function exampleVideoInference() {
  const monitor = new PerformanceMonitor();
  
  // 1. 加载视频
  const videoInfo = await optimizedLoadVideo('video.mp4');
  console.log('Video info:', videoInfo);
  
  // 2. 监听帧
  const unlistenFrame = await listenOptimizedVideoFrame((sessionId, frameIndex, boxes) => {
    monitor.recordFrame();
    
    if (frameIndex % 100 === 0) {
      console.log(`Processed ${frameIndex} frames, FPS: ${monitor.getFPS().toFixed(2)}`);
    }
  });
  
  // 3. 监听完成
  const unlistenComplete = await listenOptimizedVideoComplete((result) => {
    if (result.success) {
      console.log(`✅ Complete! Processed ${result.frames} frames`);
      console.log('Stats:', monitor.getStats());
    } else {
      console.error(`❌ Error: ${result.error}`);
    }
  });
  
  // 4. 开始推理
  const sessionId = await startOptimizedVideoInference({
    video_path: 'video.mp4',
    model_path: 'yolov8n.onnx',
    confidence: 0.5,
    frame_interval: 1,
    output_dir: './output',
  });
  
  console.log(`Started inference session: ${sessionId}`);
}

// 导出所有 API
export default {
  // Video
  optimizedLoadVideo,
  startOptimizedVideoInference,
  stopOptimizedVideoInference,
  listenOptimizedVideoFrame,
  listenOptimizedVideoComplete,
  optimizedCaptureScreenshot,
  optimizedExtractFrames,
  
  // Desktop
  optimizedGetMonitors,
  startOptimizedDesktopCapture,
  stopOptimizedDesktopCapture,
  optimizedGetActiveSessions,
  listenOptimizedDesktopFrame,
  
  // Model
  detectModelFormat,
  getModelInfo,
  checkModelCompatibility,
  getConversionInstructions,
  
  // Performance
  getInferenceSessionState,
  getDesktopCaptureStatus,
  
  // Utilities
  PerformanceMonitor,
  DetectionVisualizer,
  
  // Examples
  exampleDesktopCapture,
  exampleVideoInference,
};
