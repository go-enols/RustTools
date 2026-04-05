import type { LucideIcon } from 'lucide-react';

// 模块能力类型
export type CapabilityType =
  | 'annotation'
  | 'training'
  | 'inference'
  | 'crawling'
  | 'automation'
  | 'taskflow';

// 模块清单
export interface ModuleManifest {
  id: string;                    // 唯一标识: 'yolo', 'crawler', 'flow', 'rpa'
  name: string;                   // 显示名称: 'YOLO检测', '爬虫管理'
  icon: string;                   // Lucide icon 组件名
  description: string;            // 模块描述
  version: string;                // 模块版本
  order: number;                  // 排序顺序
  capabilities: CapabilityType[];  // 支持的能力列表
}

// 模块页面属性
export interface ModulePageProps {
  moduleId: string;
  onNavigate?: (page: string) => void;
}

// 模块接口
export interface Module {
  manifest: ModuleManifest;
  iconComponent: LucideIcon;
}

// 模块注册表状态
export interface ModuleRegistryState {
  modules: Map<string, Module>;
  activeModuleId: string | null;
}
