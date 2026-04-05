import type { Module, ModuleManifest } from './types';
import { Box } from 'lucide-react';
import * as LucideIcons from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

class ModuleRegistry {
  private modules = new Map<string, Module>();
  private listeners: Set<() => void> = new Set();

  /**
   * 注册模块
   */
  register(module: Module): void {
    if (this.modules.has(module.manifest.id)) {
      console.warn(`[ModuleRegistry] 模块 ${module.manifest.id} 已存在，将被覆盖`);
    }
    this.modules.set(module.manifest.id, module);
    this.notifyListeners();
  }

  /**
   * 卸载模块
   */
  unregister(moduleId: string): void {
    if (!this.modules.has(moduleId)) {
      console.warn(`[ModuleRegistry] 模块 ${moduleId} 不存在`);
      return;
    }
    this.modules.delete(moduleId);
    this.notifyListeners();
  }

  /**
   * 获取模块
   */
  getModule(id: string): Module | undefined {
    return this.modules.get(id);
  }

  /**
   * 获取所有已注册模块（按 order 排序）
   */
  getAllModules(): Module[] {
    return Array.from(this.modules.values()).sort(
      (a, b) => a.manifest.order - b.manifest.order
    );
  }

  /**
   * 检查模块是否已注册
   */
  hasModule(id: string): boolean {
    return this.modules.has(id);
  }

  /**
   * 获取已注册模块数量
   */
  getModuleCount(): number {
    return this.modules.size;
  }

  /**
   * 订阅变化
   */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notifyListeners(): void {
    this.listeners.forEach((l) => l());
  }
}

// 导出单例
export const moduleRegistry = new ModuleRegistry();

/**
 * 从 icon 名称获取 Lucide 图标组件
 */
export function getIconComponent(iconName: string): LucideIcon {
  const icons = LucideIcons as unknown as Record<string, LucideIcon>;
  const Icon = icons[iconName];
  if (!Icon) {
    console.warn(`[ModuleRegistry] 图标 ${iconName} 不存在，使用默认图标`);
    return Box;
  }
  return Icon;
}

/**
 * 创建模块的便捷函数
 */
export function createModule(manifest: ModuleManifest): Module {
  return {
    manifest,
    iconComponent: getIconComponent(manifest.icon),
  };
}
