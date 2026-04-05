import type { ModuleManifest } from '../types';
import { createModule, moduleRegistry } from '../registry';

export const yoloManifest: ModuleManifest = {
  id: 'yolo',
  name: 'YOLO 工具',
  icon: 'Brain',
  description: 'YOLO 一站式工具 - 数据标注、模型训练、效果测试、结果分析',
  version: '1.0.0',
  order: 10,
  capabilities: ['annotation', 'training', 'inference'],
};

// 注册 YOLO 模块
export function registerYoloModule() {
  const module = createModule(yoloManifest);
  moduleRegistry.register(module);
  return module;
}
