import { create } from 'zustand';
import {
  createProject as apiCreateProject,
  openProject as apiOpenProject,
  getRecentProjects as apiGetRecentProjects,
  saveProject as apiSaveProject,
  selectFolder,
  ProjectConfig,
} from '../api';

export interface Project {
  id: string;
  name: string;
  path: string;
  yoloVersion: string;
  classes: string[];
  trainSplit: number;
  valSplit: number;
  imageSize: number;
  description?: string;
  createdAt: Date;
}

interface WorkspaceState {
  projects: Project[];
  recentProjects: Project[];
  currentProject: Project | null;
  isLoading: boolean;
  error: string | null;

  createProject: (
    name: string,
    path: string,
    options: {
      classes: string[];
      train_split: number;
      val_split: number;
      image_size: number;
      yolo_version: string;
      description?: string;
    }
  ) => Promise<void>;

  openProject: (project: Project) => void;
  openProjectFromPath: (projectPath: string) => Promise<boolean>;
  closeProject: () => void;
  loadRecentProjects: () => Promise<void>;
  loadCurrentProject: () => void;
  clearRecentProjects: () => void;
  saveCurrentProject: () => Promise<void>;
  selectProjectPath: () => Promise<string | null>;
  setError: (error: string | null) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  projects: [],
  recentProjects: [],
  currentProject: null,
  isLoading: false,
  error: null,

  createProject: async (name, path, options) => {
    set({ isLoading: true, error: null });
    try {
      // Call backend API
      const config: ProjectConfig = {
        name,
        path,
        yolo_version: options.yolo_version as 'yolo5' | 'yolo8' | 'yolo11',
        classes: options.classes,
        train_split: options.train_split,
        val_split: options.val_split,
        image_size: options.image_size,
        description: options.description,
      };

      const response = await apiCreateProject(config);

      if (!response.success || !response.data) {
        throw new Error(response.error || '创建项目失败');
      }

      const config_data = response.data;
      const newProject: Project = {
        id: crypto.randomUUID(),
        name: config_data.name,
        path: config_data.path,
        yoloVersion: config_data.yolo_version,
        classes: config_data.classes,
        trainSplit: config_data.train_split,
        valSplit: config_data.val_split,
        imageSize: config_data.image_size,
        description: config_data.description,
        createdAt: new Date(),
      };

      set((state) => ({
        projects: [...state.projects, newProject],
        recentProjects: [newProject, ...state.recentProjects.slice(0, 9)],
        currentProject: newProject,
        isLoading: false,
      }));

      // Save to localStorage for recent projects
      const updatedRecent = [newProject, ...get().recentProjects.slice(0, 9)];
      localStorage.setItem('recentProjects', JSON.stringify(updatedRecent));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '创建项目失败', isLoading: false });
    }
  },

  openProject: (project) => {
    set({ currentProject: project });
    const { recentProjects } = get();
    const filtered = recentProjects.filter((p) => p.id !== project.id);
    const updated = [project, ...filtered];
    set({ recentProjects: updated });
    localStorage.setItem('recentProjects', JSON.stringify(updated));
    localStorage.setItem('currentProject', JSON.stringify(project));
  },

  openProjectFromPath: async (projectPath: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await apiOpenProject(projectPath);
      if (!response.success || !response.data) {
        throw new Error(response.error || '打开项目失败');
      }

      const config = response.data;
      const project: Project = {
        id: crypto.randomUUID(),
        name: config.name,
        path: config.path,
        yoloVersion: config.yolo_version,
        classes: config.classes,
        trainSplit: config.train_split,
        valSplit: config.val_split,
        imageSize: config.image_size,
        description: config.description,
        createdAt: new Date(),
      };

      set((state) => ({
        currentProject: project,
        recentProjects: [project, ...state.recentProjects.filter((p) => p.path !== projectPath)].slice(0, 10),
        isLoading: false,
      }));

      localStorage.setItem('recentProjects', JSON.stringify(get().recentProjects));
      return true;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '打开项目失败', isLoading: false });
      return false;
    }
  },

  closeProject: () => {
    set({ currentProject: null });
    localStorage.removeItem('currentProject');
  },

  loadRecentProjects: async () => {
    // First try to load from backend
    const response = await apiGetRecentProjects();
    if (response.success && response.data && response.data.length > 0) {
      // TODO: Load full project configs from backend
      set({ recentProjects: [] });
    }

    // Fallback to localStorage
    const saved = localStorage.getItem('recentProjects');
    if (saved) {
      try {
        const projects = JSON.parse(saved);
        set({ recentProjects: projects });
      } catch {
        set({ recentProjects: [] });
      }
    }
  },

  clearRecentProjects: () => {
    localStorage.removeItem('recentProjects');
    set({ recentProjects: [] });
  },

  loadCurrentProject: () => {
    const saved = localStorage.getItem('currentProject');
    if (saved) {
      try {
        const project = JSON.parse(saved);
        set({ currentProject: project });
      } catch {
        localStorage.removeItem('currentProject');
      }
    }
  },

  saveCurrentProject: async () => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      const response = await apiSaveProject();
      if (!response.success) {
        throw new Error(response.error || '保存项目失败');
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : '保存项目失败' });
    }
  },

  selectProjectPath: async () => {
    const result = await selectFolder('选择项目文件夹', 'D:\\');
    if (result.canceled || !result.path) {
      return null;
    }
    return result.path;
  },

  setError: (error) => set({ error }),
}));
