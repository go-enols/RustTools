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
