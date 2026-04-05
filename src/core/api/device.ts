/**
 * Device API - Device management
 */
import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse, DeviceInfo } from './types';

/**
 * Get list of available devices
 */
export async function getDevices(): Promise<ApiResponse<DeviceInfo[]>> {
  try {
    return await invoke<ApiResponse<DeviceInfo[]>>('device_list');
  } catch (error) {
    console.error('[API] getDevices error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Get device utilization stats
 */
export async function getDeviceStats(deviceId: number): Promise<ApiResponse<{ gpu_util: number; memory_util: number; temperature: number }>> {
  try {
    return await invoke<ApiResponse<{ gpu_util: number; memory_util: number; temperature: number }>>('device_stats', { deviceId });
  } catch (error) {
    console.error('[API] getDeviceStats error:', error);
    return { success: false, error: String(error) };
  }
}

/**
 * Set default training device
 */
export async function setDefaultDevice(deviceId: number): Promise<ApiResponse<void>> {
  try {
    return await invoke<ApiResponse<void>>('device_set_default', { deviceId });
  } catch (error) {
    console.error('[API] setDefaultDevice error:', error);
    return { success: false, error: String(error) };
  }
}
