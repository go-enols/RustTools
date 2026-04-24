import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Home } from "lucide-react";

export default function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    const update = async () => setIsMaximized(await win.isMaximized());
    update();
    const unlisten = win.listen("tauri://resize", update);
    return () => { unlisten.then((f) => f()); };
  }, []);

  const onMinimize = async () => {
    try { await getCurrentWindow().minimize(); } catch (e) { console.error(e); }
  };

  const onMaximize = async () => {
    try {
      const win = getCurrentWindow();
      if (await win.isMaximized()) {
        await win.unmaximize();
      } else {
        await win.maximize();
      }
    } catch (e) { console.error(e); }
  };

  const onClose = async () => {
    try { await getCurrentWindow().close(); } catch (e) { console.error(e); }
  };

  const onDragMouseDown = async (e: React.MouseEvent) => {
    // 只有左键才触发拖拽
    if (e.button !== 0) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (e) {
      console.error("startDragging failed:", e);
    }
  };

  // 阻止窗口按钮的 mousedown 事件冒泡，避免触发 Tauri 拖拽导致 click 失效
  const stopPropagation = (e: React.MouseEvent) => e.stopPropagation();

  return (
    <div
      data-tauri-drag-region
      onMouseDown={onDragMouseDown}
      className="h-8 shrink-0 bg-surface dark:bg-surface-dark border-b border-gray-200 dark:border-gray-800 flex items-center justify-between select-none z-50"
    >
      {/* 左侧：标题（可拖拽） */}
      <div className="flex items-center gap-2 px-3 h-full" data-tauri-drag-region onMouseDown={onDragMouseDown}>
        <div className="w-4 h-4 rounded-sm bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
          <span className="text-[8px] font-bold text-white">R</span>
        </div>
        <span className="text-xs text-gray-700 dark:text-gray-200">RustTools</span>
      </div>

      {/* 中间：可拖拽区域 */}
      <div className="flex-1 h-full" data-tauri-drag-region onMouseDown={onDragMouseDown} />

      {/* 右侧：Home + Windows 风格窗口按钮 */}
      <div className="flex items-center h-full">
        <button
          onClick={() => { window.location.hash = "#/"; }}
          onMouseDown={stopPropagation}
          className="w-10 h-full flex items-center justify-center text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
          title="首页"
        >
          <Home className="w-4 h-4" strokeWidth={1.8} />
        </button>
        <button
          onClick={onMinimize}
          onMouseDown={stopPropagation}
          className="w-10 h-full flex items-center justify-center text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
          title="最小化"
        >
          <svg width="10" height="1" viewBox="0 0 10 1">
            <rect width="10" height="1" fill="currentColor" />
          </svg>
        </button>
        <button
          onClick={onMaximize}
          onMouseDown={stopPropagation}
          className="w-10 h-full flex items-center justify-center text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
          title={isMaximized ? "还原" : "最大化"}
        >
          {isMaximized ? (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <rect x="0.5" y="2.5" width="7" height="7" stroke="currentColor" fill="none" strokeWidth="1" />
              <rect x="2.5" y="0.5" width="7" height="7" stroke="currentColor" fill="none" strokeWidth="1" />
            </svg>
          ) : (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" fill="none" strokeWidth="1" />
            </svg>
          )}
        </button>
        <button
          onClick={onClose}
          onMouseDown={stopPropagation}
          className="w-10 h-full flex items-center justify-center text-gray-500 dark:text-gray-400 hover:bg-red-500 hover:text-white transition"
          title="关闭"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" strokeWidth="1" fill="none" />
          </svg>
        </button>
      </div>
    </div>
  );
}
