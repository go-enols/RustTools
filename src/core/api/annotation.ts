/**
 * Annotation API - Image annotation
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, AnnotationImage, AnnotationBox } from './types';

/**
 * Load image for annotation
 */
export async function loadAnnotationImage(imagePath: string): Promise<ApiResponse<AnnotationImage>> {
  try {
    return await invoke<ApiResponse<AnnotationImage>>('annotation_load_image', { imagePath });
  } catch (error) {
    console.error('[API] loadAnnotationImage error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Save annotation for image
 */
export async function saveAnnotations(imageId: string, annotations: AnnotationBox[]): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('annotation_save', { imageId, annotations });
  } catch (error) {
    console.error('[API] saveAnnotations error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get next/previous image in dataset
 */
export async function navigateImage(currentPath: string, direction: 'next' | 'previous'): Promise<ApiResponse<AnnotationImage | null>> {
  try {
    return await invoke<ApiResponse<AnnotationImage | null>>('annotation_navigate', { currentPath, direction });
  } catch (error) {
    console.error('[API] navigateImage error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Auto-label images using pretrained model
 */
export async function autoLabel(modelPath: string, confidence: number): Promise<ApiResponse<{ labeled: number }>> {
  try {
    return await invoke<ApiResponse<{ labeled: number }>>('annotation_auto_label', { modelPath, confidence });
  } catch (error) {
    console.error('[API] autoLabel error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Add new class to dataset
 */
export async function addClass(name: string, color: string): Promise<ApiResponse<{ class_id: number }>> {
  try {
    return await invoke<ApiResponse<{ class_id: number }>>('annotation_add_class', { name, color });
  } catch (error) {
    console.error('[API] addClass error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Delete class from dataset
 */
export async function deleteClass(classId: number): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('annotation_delete_class', { classId });
  } catch (error) {
    console.error('[API] deleteClass error:', error);
    return { success: false, error: String(error) };
  }
}

// YOLO Project Classes Management

export interface YoloAnnotation {
  class_id: number;
  x_center: number;
  y_center: number;
  width: number;
  height: number;
}

export interface UpdateClassesResponse {
  success: boolean;
  data?: never;
  error?: string;
}

export interface LoadAnnotationResponse {
  success: boolean;
  data?: YoloAnnotation[];
  error?: string;
}

/**
 * Update project classes in project.yaml
 */
export async function updateClasses(
  projectPath: string,
  classes: string[]
): Promise<UpdateClassesResponse> {
  try {
    return await invoke<UpdateClassesResponse>('update_classes', { projectPath, classes });
  } catch (error) {
    console.error('[API] updateClasses error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Load annotations from a YOLO label file
 */
export async function loadAnnotation(
  labelPath: string
): Promise<LoadAnnotationResponse> {
  try {
    return await invoke<LoadAnnotationResponse>('load_annotation', { labelPath });
  } catch (error) {
    console.error('[API] loadAnnotation error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Save annotations to a YOLO label file
 */
export async function saveAnnotation(
  labelPath: string,
  annotations: YoloAnnotation[]
): Promise<UpdateClassesResponse> {
  try {
    return await invoke<UpdateClassesResponse>('save_annotation', { labelPath, annotations });
  } catch (error) {
    console.error('[API] saveAnnotation error:', error);
    return { success: false, error: String(error) };
  }
}
