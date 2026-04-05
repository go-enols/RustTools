import { create } from 'zustand';
import {
  loadSettings as apiLoadSettings,
  saveSettings as apiSaveSettings,
  getDevices as apiGetDevices,
  setDefaultDevice as apiSetDefaultDevice,
  DeviceInfo,
} from '../api';

export type { DeviceInfo } from '../api';

export interface Settings {
  theme: 'dark' | 'light';
  language: 'zh' | 'en';
  autoSaveMinutes: number;
  openRecentOnStartup: boolean;
  animationsEnabled: boolean;
  defaultDevice: string;
  fallbackDevice: string;
  vramLimitGb: number;
  workers: number;
  datasetPath: string;
  modelPath: string;
  cachePath: string;
}

const defaultSettings: Settings = {
  theme: 'dark',
  language: 'zh',
  autoSaveMinutes: 5,
  openRecentOnStartup: true,
  animationsEnabled: true,
  defaultDevice: 'GPU 0',
  fallbackDevice: 'CPU',
  vramLimitGb: 8,
  workers: 8,
  datasetPath: 'D:\\datasets',
  modelPath: 'D:\\models',
  cachePath: 'D:\\cache',
};

interface SettingsState {
  settings: Settings;
  devices: DeviceInfo[];
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;

  loadSettings: () => Promise<void>;
  saveSettings: (settings: Settings) => Promise<void>;
  updateSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  resetToDefaults: () => void;
  loadDevices: () => Promise<void>;
  setDefaultDevice: (deviceId: number) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: defaultSettings,
  devices: [],
  isLoading: false,
  isSaving: false,
  error: null,

  loadSettings: async () => {
    set({ isLoading: true });

    try {
      // Try to load from backend first
      const response = await apiLoadSettings();
      if (response.success && response.data) {
        const merged = { ...defaultSettings, ...response.data } as Settings;
        set({ settings: merged, isLoading: false });
        return;
      }
    } catch (error) {
      console.warn('Failed to load settings from backend, using localStorage:', error);
    }

    // Fallback to localStorage
    const saved = localStorage.getItem('settings');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        set({ settings: { ...defaultSettings, ...parsed }, isLoading: false });
      } catch {
        set({ settings: defaultSettings, isLoading: false });
      }
    } else {
      set({ isLoading: false });
    }
  },

  saveSettings: async (settings) => {
    set({ isSaving: true, error: null });

    try {
      // Try to save to backend first
      const response = await apiSaveSettings(settings as unknown as Record<string, unknown>);
      if (!response.success) {
        throw new Error(response.error || '保存设置失败');
      }
      set({ settings, isSaving: false });
    } catch (error) {
      console.warn('Failed to save settings to backend, using localStorage:', error);
      // Fallback to localStorage
      try {
        localStorage.setItem('settings', JSON.stringify(settings));
        set({ settings, isSaving: false });
      } catch {
        set({ error: '保存设置失败', isSaving: false });
      }
    }
  },

  updateSetting: (key, value) => {
    set((state) => ({
      settings: { ...state.settings, [key]: value },
    }));
  },

  resetToDefaults: () => {
    set({ settings: defaultSettings });
    localStorage.removeItem('settings');
    apiSaveSettings(defaultSettings as unknown as Record<string, unknown>).catch(console.error);
  },

  loadDevices: async () => {
    try {
      const response = await apiGetDevices();
      if (response.success && response.data) {
        set({ devices: response.data });
      }
    } catch (error) {
      console.error('Failed to load devices:', error);
      // Set default devices as fallback
      set({
        devices: [
          {
            id: 0,
            name: 'NVIDIA GeForce RTX 3080',
            type: 'GPU',
            memory_total: 10737418240,
            memory_used: 2147483648,
            memory_free: 8589934592,
            driver_version: '536.23',
            cuda_version: '12.2',
            compute_capability: '8.6',
          },
          {
            id: 1,
            name: 'Intel Core i9-12900K',
            type: 'CPU',
            memory_total: 34359738368,
            memory_used: 8589934592,
            memory_free: 25769803776,
          },
        ],
      });
    }
  },

  setDefaultDevice: async (deviceId) => {
    try {
      await apiSetDefaultDevice(deviceId);
      const { devices } = get();
      const device = devices.find((d) => d.id === deviceId);
      if (device) {
        set((state) => ({
          settings: { ...state.settings, defaultDevice: device.name },
        }));
      }
    } catch (error) {
      console.error('Failed to set default device:', error);
    }
  },
}));
