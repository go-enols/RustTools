// Model Conversion API - Pure Rust Backend
import { invoke } from '@tauri-apps/api/core';

// Types
export interface CommandResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

export interface FormatInfo {
  format: string;
  name: string;
  description: string;
  can_optimize: boolean;
  can_quantize: boolean;
  can_simplify: boolean;
}

export interface SupportedFormatsResponse {
  rust_supported: FormatInfo[];
  python_required: FormatInfo[];
}

export interface FormatDetectionResult {
  format: string;
  extension: string;
  can_convert_in_rust: boolean;
  requires_python: boolean;
}

export interface ModelDetailsResponse {
  format: string;
  path: string;
  file_size: string;
  is_compatible: boolean;
  details: string;
}

export interface CompatibilityCheckResult {
  is_compatible: boolean;
  format: string;
  message: string;
  conversion_hint: string | null;
  can_use_in_rust: boolean;
}

export interface ConversionGuideResponse {
  format: string;
  instructions: string;
  requires_python: boolean;
  python_note: string | null;
}

export interface SimplificationResult {
  success: boolean;
  input_path: string;
  output_path: string;
  message: string;
}

export interface OptimizationResult {
  success: boolean;
  input_path: string;
  output_path: string;
  message: string;
}

export interface ScriptPathResponse {
  exists: boolean;
  path: string | null;
  usage: string;
}

/**
 * Get supported conversion formats
 */
export async function getSupportedFormats(): Promise<CommandResponse<SupportedFormatsResponse>> {
  return invoke('get_supported_formats');
}

/**
 * Detect model format
 */
export async function detectModelFormat(path: string): Promise<CommandResponse<FormatDetectionResult>> {
  return invoke('detect_format', { path });
}

/**
 * Get detailed model information
 */
export async function getModelDetails(path: string): Promise<CommandResponse<ModelDetailsResponse>> {
  return invoke('get_model_details', { path });
}

/**
 * Check model compatibility
 */
export async function checkModelCompatibility(path: string): Promise<CommandResponse<CompatibilityCheckResult>> {
  return invoke('check_compatibility', { path });
}

/**
 * Simplify ONNX model (Pure Rust)
 * Note: Due to tract limitations, this validates but cannot save back as ONNX
 */
export async function simplifyONNXModel(
  modelPath: string,
  outputPath?: string
): Promise<CommandResponse<SimplificationResult>> {
  return invoke('simplify_onnx_model', {
    path: modelPath,
    outputPath: outputPath || null
  });
}

/**
 * Optimize ONNX model (Pure Rust)
 * Note: Due to tract limitations, this optimizes internally but cannot save back as ONNX
 */
export async function optimizeONNXModel(
  modelPath: string,
  outputPath?: string
): Promise<CommandResponse<OptimizationResult>> {
  return invoke('optimize_onnx_model', {
    path: modelPath,
    outputPath: outputPath || null
  });
}

/**
 * Get conversion instructions
 */
export async function getConversionGuide(path: string): Promise<CommandResponse<ConversionGuideResponse>> {
  return invoke('get_conversion_guide', { path });
}

/**
 * Get Python conversion script path
 */
export async function getConversionScriptPath(): Promise<CommandResponse<ScriptPathResponse>> {
  return invoke('get_conversion_script_path');
}
