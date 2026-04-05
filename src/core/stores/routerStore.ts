import { create } from 'zustand';

export type PageType = 'hub' | 'yolo' | 'annotation' | 'training' | 'results' | 'video' | 'device' | 'tools' | 'settings';

interface RouterState {
  // 当前活动模块 ID
  activeModuleId: string | null;
  // 当前页面
  activePage: PageType;
  // 路由参数
  params: Record<string, string>;
  // 页面历史
  history: Array<{ moduleId: string | null; page: PageType }>;

  // Actions
  navigateToModule: (moduleId: string | null, page?: PageType) => void;
  navigateToPage: (page: PageType) => void;
  setParams: (params: Record<string, string>) => void;
  goBack: () => void;
  goToHub: () => void;
}

export const useRouterStore = create<RouterState>((set, get) => ({
  activeModuleId: null,
  activePage: 'hub',
  params: {},
  history: [],

  navigateToModule: (moduleId, page = 'yolo') => {
    const { history } = get();
    set({
      activeModuleId: moduleId,
      activePage: page,
      history: [...history, { moduleId, page }],
      params: {},
    });
  },

  navigateToPage: (page) => {
    const { activeModuleId, history } = get();
    set({
      activePage: page,
      history: [...history, { moduleId: activeModuleId, page }],
    });
  },

  setParams: (params) => {
    set({ params });
  },

  goBack: () => {
    const { history } = get();
    if (history.length > 1) {
      const newHistory = history.slice(0, -1);
      const prev = newHistory[newHistory.length - 1];
      set({
        history: newHistory,
        activeModuleId: prev.moduleId,
        activePage: prev.page,
      });
    }
  },

  goToHub: () => {
    const { history } = get();
    set({
      activeModuleId: null,
      activePage: 'hub',
      history: [...history, { moduleId: null, page: 'hub' }],
      params: {},
    });
  },
}));
