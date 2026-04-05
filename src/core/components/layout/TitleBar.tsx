import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X, Maximize2 } from 'lucide-react';
import { useState, useEffect } from 'react';

export default function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const checkMaximized = async () => {
      const maximized = await getCurrentWindow().isMaximized();
      setIsMaximized(maximized);
    };
    checkMaximized();

    // Listen for window state changes
    const unlisten = getCurrentWindow().onResized(() => {
      checkMaximized();
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  const handleMinimize = () => {
    getCurrentWindow().minimize();
  };

  const handleMaximize = async () => {
    const window = getCurrentWindow();
    if (isMaximized) {
      await window.unmaximize();
    } else {
      await window.maximize();
    }
    setIsMaximized(!isMaximized);
  };

  const handleClose = () => {
    getCurrentWindow().close();
  };

  const handleDragStart = (e: React.MouseEvent) => {
    // Only start drag if clicking on the drag area (not on buttons)
    if ((e.target as HTMLElement).closest('.title-bar-btn')) {
      return;
    }
    getCurrentWindow().startDragging();
  };

  const handleDoubleClick = (e: React.MouseEvent) => {
    // Double click on drag area to toggle maximize
    if (!(e.target as HTMLElement).closest('.title-bar-btn')) {
      handleMaximize();
    }
  };

  return (
    <div
      className="title-bar"
      onMouseDown={handleDragStart}
      onDoubleClick={handleDoubleClick}
    >
      <div className="title-bar-drag">
        <div className="title-bar-icon">Y</div>
        <span className="title-bar-title">YOLO-Flow</span>
      </div>

      <div className="title-bar-controls">
        <button
          className="title-bar-btn title-bar-btn-minimize"
          onClick={handleMinimize}
          title="最小化"
        >
          <Minus size={14} />
        </button>
        <button
          className="title-bar-btn title-bar-btn-maximize"
          onClick={handleMaximize}
          title={isMaximized ? "还原" : "最大化"}
        >
          {isMaximized ? <Square size={12} /> : <Maximize2 size={12} />}
        </button>
        <button
          className="title-bar-btn title-bar-btn-close"
          onClick={handleClose}
          title="关闭"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
