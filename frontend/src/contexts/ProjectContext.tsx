import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";

export interface DatasetPaths {
  train: string;
  val: string;
}

export interface ProjectConfig {
  name: string;
  path: string;
  yolo_version: string;
  classes: string[];
  train_split: number;
  val_split: number;
  image_size: number;
  images: DatasetPaths;
  labels: DatasetPaths;
}

export interface ProjectScan {
  images: number;
  labels: number;
}

interface ProjectContextValue {
  project: ProjectConfig | null;
  scan: ProjectScan | null;
  loading: boolean;
  refresh: () => Promise<void>;
  openProject: (path: string) => Promise<void>;
}

const ProjectContext = createContext<ProjectContextValue | null>(null);

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [project, setProject] = useState<ProjectConfig | null>(null);
  const [scan, setScan] = useState<ProjectScan | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const p = await invoke<ProjectConfig | null>("get_current_project");
      setProject(p);
      if (p) {
        const s = await invoke<{
          train_images: number;
          val_images: number;
          total_annotations: number;
        }>("scan_project", { path: p.path });
        setScan({ images: s.train_images + s.val_images, labels: s.total_annotations });
      } else {
        setScan(null);
      }
    } catch (e) {
      console.error("refresh project failed:", e);
      setProject(null);
      setScan(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const openProject = useCallback(
    async (path: string) => {
      await invoke("open_project", { path });
      await refresh();
    },
    [refresh]
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <ProjectContext.Provider value={{ project, scan, loading, refresh, openProject }}>
      {children}
    </ProjectContext.Provider>
  );
}

export function useProject() {
  const ctx = useContext(ProjectContext);
  if (!ctx) {
    throw new Error("useProject must be used within ProjectProvider");
  }
  return ctx;
}
