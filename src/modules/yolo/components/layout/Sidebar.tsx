import { useState, useEffect } from 'react';
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  File,
  RefreshCw,
  FolderPlus,
  FilePlus,
  Trash2,
} from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { PageType } from '../../../../core/stores/routerStore';
import { useWorkspaceStore } from '../../../../core/stores/workspaceStore';
import { listDirectory, FileInfo, startWatch, stopWatch, FileChangeEvent, writeTextFile, createDirectory, deleteFile, deleteDirectory } from '../../../../core/api/file';
import { InputModal, ConfirmModal } from '../../../../shared/components/ui/Modal';

interface SidebarProps {
  currentPage: PageType;
  activeSidebar: 'explorer' | 'search' | 'none';
  onOpenFile: (fileName: string, filePath: string) => void;
}

// File tree node interface
interface FileNode {
  name: string;
  path: string;
  type: 'file' | 'folder';
  children?: FileNode[];
  expanded?: boolean;
  loading?: boolean;
}

export default function Sidebar({ currentPage, activeSidebar, onOpenFile }: SidebarProps) {
  const { currentProject } = useWorkspaceStore();
  const [fileTree, setFileTree] = useState<FileNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; node: FileNode | null } | null>(null);
  const [modalState, setModalState] = useState<{
    type: 'createFile' | 'createFolder' | 'delete' | null;
    targetPath?: string;
    node?: FileNode;
  }>({ type: null });

  // Close context menu on click outside
  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    document.addEventListener('click', handleClick);
    return () => document.removeEventListener('click', handleClick);
  }, []);


  // Create new file
  const handleCreateFile = async (parentPath: string, name: string) => {
    const filePath = `${parentPath}\\${name}`;
    const result = await writeTextFile(filePath, '');
    if (result.success) {
      updateNodeChildren(parentPath, await loadDirectory(parentPath));
    }
    setModalState({ type: null });
  };

  // Create new folder
  const handleCreateFolder = async (parentPath: string, name: string) => {
    const folderPath = `${parentPath}\\${name}`;
    const result = await createDirectory(folderPath);
    if (result.success) {
      updateNodeChildren(parentPath, await loadDirectory(parentPath));
    }
    setModalState({ type: null });
  };

  // Delete file or folder
  const handleDelete = async (node: FileNode) => {
    const parentPath = node.path.substring(0, node.path.lastIndexOf('\\'));
    let result;
    if (node.type === 'folder') {
      result = await deleteDirectory(node.path);
    } else {
      result = await deleteFile(node.path);
    }
    if (result.success) {
      updateNodeChildren(parentPath, await loadDirectory(parentPath));
    }
    setModalState({ type: null });
  };

  // Load root directory when project changes
  useEffect(() => {
    if (currentProject?.path) {
      loadDirectory(currentProject.path).then(setFileTree);
    } else {
      setFileTree([]);
    }
  }, [currentProject?.path]);

  // Start/stop file watcher when project changes
  useEffect(() => {
    if (!currentProject?.path) return;

    let unlisten: (() => void) | null = null;

    const setupWatcher = async () => {
      // Start watching the project directory
      await startWatch(currentProject.path);

      // Listen for file change events
      unlisten = await listen<FileChangeEvent>('file-change', async (event) => {
        console.log('[Sidebar] File change detected:', event.payload);
        // Incremental update: refresh only the parent directory
        const children = await loadDirectory(event.payload.parent);
        updateNodeChildren(event.payload.parent, children);
      });
    };

    setupWatcher();

    return () => {
      if (currentProject?.path) {
        stopWatch(currentProject.path);
      }
      if (unlisten) {
        unlisten();
      }
    };
  }, [currentProject?.path]);

  // Update a specific node's children by path - preserves the exact node reference
  const updateNodeChildren = (targetPath: string, newChildren: FileNode[]): void => {
    setFileTree((prev) => {
      // If targetPath is the project root, replace entire tree
      if (currentProject?.path && (targetPath === currentProject.path || targetPath === currentProject.path + '\\')) {
        return newChildren;
      }

      const newTree = JSON.parse(JSON.stringify(prev)) as FileNode[];

      // Find and update the node
      const findAndUpdate = (nodes: FileNode[]): boolean => {
        for (let i = 0; i < nodes.length; i++) {
          if (nodes[i].path === targetPath) {
            // Preserve the existing node's expanded state and merge children
            const existingNode = nodes[i];
            // Update children while preserving expanded state
            const updatedChildren = newChildren.map(child => {
              // Check if this child was previously expanded
              const existingChild = existingNode.children?.find(c => c.path === child.path);
              if (existingChild && child.type === 'folder') {
                return {
                  ...child,
                  expanded: existingChild.expanded,
                  children: existingChild.children,
                };
              }
              return child;
            });
            nodes[i].children = updatedChildren;
            return true;
          }
          if (nodes[i].children && findAndUpdate(nodes[i].children!)) {
            return true;
          }
        }
        return false;
      };

      findAndUpdate(newTree);
      return newTree;
    });
  };

  const loadDirectory = async (dirPath: string): Promise<FileNode[]> => {
    const result = await listDirectory(dirPath);
    if (result.success && result.data) {
      return result.data.map((item: FileInfo) => ({
        name: item.name,
        path: item.path,
        type: item.is_dir ? 'folder' : 'file',
        expanded: false,
        children: undefined,
      }));
    }
    return [];
  };

  // Collect all expanded folder paths from current tree
  const getExpandedPaths = (nodes: FileNode[]): Set<string> => {
    const expanded = new Set<string>();
    for (const node of nodes) {
      if (node.type === 'folder' && node.expanded) {
        // node.path is already the full filesystem path
        expanded.add(node.path);
        if (node.children) {
          const childExpanded = getExpandedPaths(node.children);
          childExpanded.forEach(p => expanded.add(p));
        }
      }
    }
    return expanded;
  };

  // Restore expanded state in new tree based on saved paths
  const restoreExpandedState = (nodes: FileNode[], expandedPaths: Set<string>): FileNode[] => {
    return nodes.map(node => {
      if (node.type === 'folder') {
        const isExpanded = expandedPaths.has(node.path);
        return {
          ...node,
          expanded: isExpanded,
          children: node.children ? restoreExpandedState(node.children, expandedPaths) : undefined,
        };
      }
      return node;
    });
  };

  // Load children for expanded folders that don't have children yet
  const loadChildrenForExpandedFolders = (nodes: FileNode[]): void => {
    const traverse = (nodes: FileNode[]) => {
      for (const node of nodes) {
        if (node.type === 'folder' && node.expanded && node.children === undefined) {
          // Trigger async load and update when done
          loadDirectory(node.path).then((children) => {
            updateNodeChildren(node.path, children);
          });
        }
        if (node.children) {
          traverse(node.children);
        }
      }
    };

    traverse(nodes);
  };

  const refreshTree = async () => {
    if (!currentProject?.path) return;
    setLoading(true);

    // Save expanded paths before refreshing
    const expandedPaths = getExpandedPaths(fileTree);

    // Load fresh data and restore expanded state
    const children = await loadDirectory(currentProject.path);
    const childrenWithState = restoreExpandedState(children, expandedPaths);
    setFileTree(childrenWithState);
    setLoading(false);

    // Load children for folders that were expanded but have undefined children
    loadChildrenForExpandedFolders(childrenWithState);
  };

  const toggleFolder = (path: number[]) => {
    setFileTree((prev) => {
      const newTree = JSON.parse(JSON.stringify(prev)) as FileNode[];
      let current = newTree;
      for (let i = 0; i < path.length - 1; i++) {
        current = current[path[i]].children!;
      }
      const lastIndex = path[path.length - 1];
      const node = current[lastIndex];
      const wasExpanded = node.expanded;
      node.expanded = !node.expanded;

      // If expanding and children not yet loaded (undefined), trigger load
      if (!wasExpanded && node.children === undefined) {
        // Load children asynchronously
        loadDirectory(node.path).then((children) => {
          setFileTree((prev) => {
            const newTree = JSON.parse(JSON.stringify(prev)) as FileNode[];
            let current = newTree;
            for (let i = 0; i < path.length - 1; i++) {
              current = current[path[i]].children!;
            }
            current[path[path.length - 1]].children = children;
            return newTree;
          });
        });
      }

      return newTree;
    });
  };

  const renderNode = (node: FileNode, path: number[], depth: number = 0) => {
    const isFolder = node.type === 'folder';
    const Icon = isFolder ? (node.expanded ? FolderOpen : Folder) : File;

    const handleClick = () => {
      if (isFolder) {
        toggleFolder(path);
      } else {
        onOpenFile(node.name, node.path);
      }
    };

    const handleContextMenu = (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setContextMenu({ x: e.clientX, y: e.clientY, node });
    };

    return (
      <div key={node.path} className="file-tree-node">
        <div
          className="file-tree-item"
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          onClick={handleClick}
          onContextMenu={handleContextMenu}
        >
          {isFolder && (
            <span className="file-tree-chevron">
              {node.expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            </span>
          )}
          <Icon size={16} className="file-tree-icon" />
          <span className="file-tree-name">{node.name}</span>
        </div>
        {isFolder && node.expanded && node.children && (
          <div className="file-tree-children">
            {node.children.map((child, index) => renderNode(child, [...path, index], depth + 1))}
          </div>
        )}
      </div>
    );
  };

  if (activeSidebar === 'none') {
    return null;
  }

  return (
    <aside className="sidebar sidebar-explorer">
      <div className="sidebar-header">
        <div className="sidebar-actions">
          <button className="sidebar-action-btn" onClick={refreshTree} title="刷新">
            <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      <div
        className="sidebar-content"
        onContextMenu={(e) => {
          const target = e.target as HTMLElement;
          // 如果点击的是 file-tree-item，让 renderNode 里的处理
          if (target.closest('.file-tree-item')) {
            return;
          }
          // 空白区域右键，显示新建菜单
          e.preventDefault();
          setContextMenu({ x: e.clientX, y: e.clientY, node: null });
        }}
      >
        <div className="explorer-section">
          {loading && fileTree.length === 0 ? (
            <div className="file-tree-empty">
              {currentProject ? '项目文件夹为空' : '未打开项目'}
            </div>
          ) : fileTree.length === 0 ? (
            <div className="file-tree-empty">
              {currentProject ? '项目文件夹为空' : '未打开项目'}
            </div>
          ) : (
            <div className="file-tree">
              {fileTree.map((node, index) => renderNode(node, [index]))}
            </div>
          )}

          {/* Context Menu */}
          {contextMenu && (
            <div
              className="context-menu"
              style={{ left: contextMenu.x, top: contextMenu.y }}
              onClick={(e) => e.stopPropagation()}
            >
              {/* 新建文件/文件夹 - 任何节点或空白区域 */}
              {currentProject && (
                <>
                  <button
                    className="context-menu-item"
                    onClick={() => {
                      // 如果是文件节点，使用父目录；否则使用节点路径
                      const targetPath = contextMenu.node?.type === 'file'
                        ? contextMenu.node.path.substring(0, contextMenu.node.path.lastIndexOf('\\'))
                        : (contextMenu.node?.path || currentProject?.path);
                      setContextMenu(null);
                      setModalState({ type: 'createFile', targetPath });
                    }}
                  >
                    <FilePlus size={14} />
                    <span>新建文件</span>
                  </button>
                  <button
                    className="context-menu-item"
                    onClick={() => {
                      const targetPath = contextMenu.node?.path || currentProject?.path;
                      setContextMenu(null);
                      setModalState({ type: 'createFolder', targetPath });
                    }}
                  >
                    <FolderPlus size={14} />
                    <span>新建文件夹</span>
                  </button>
                  {contextMenu.node && <div className="context-menu-divider" />}
                </>
              )}
              {/* 删除 - 仅节点有值时显示 */}
              {contextMenu.node && (
                <button
                  className="context-menu-item"
                  onClick={() => {
                    const nodeToDelete = contextMenu.node!;
                    setContextMenu(null);
                    setModalState({ type: 'delete', node: nodeToDelete });
                  }}
                >
                  <Trash2 size={14} />
                  <span>删除</span>
                </button>
              )}
            </div>
          )}

          {/* Create File Modal */}
          <InputModal
            isOpen={modalState.type === 'createFile'}
            onClose={() => setModalState({ type: null })}
            onConfirm={(name) => handleCreateFile(modalState.targetPath!, name)}
            title="新建文件"
            label="文件名"
            placeholder="请输入文件名"
          />

          {/* Create Folder Modal */}
          <InputModal
            isOpen={modalState.type === 'createFolder'}
            onClose={() => setModalState({ type: null })}
            onConfirm={(name) => handleCreateFolder(modalState.targetPath!, name)}
            title="新建文件夹"
            label="文件夹名"
            placeholder="请输入文件夹名"
          />

          {/* Delete Confirm Modal */}
          <ConfirmModal
            isOpen={modalState.type === 'delete'}
            onClose={() => setModalState({ type: null })}
            onConfirm={() => handleDelete(modalState.node!)}
            title="确认删除"
            message={`确定要删除 "${modalState.node?.name}" 吗?`}
            confirmText="删除"
            variant="danger"
          />
        </div>
      </div>

      <div className="sidebar-footer">
        <div className="sidebar-footer-info">
          {currentPage !== 'hub' && (
            <span className="current-page-indicator">{getPageLabel(currentPage)}</span>
          )}
        </div>
      </div>
    </aside>
  );
}

function getPageLabel(page: PageType): string {
  const labels: Record<PageType, string> = {
    hub: '首页',
    yolo: 'YOLO',
    annotation: '标注',
    training: '训练',
    results: '结果',
    video: '视频',
    desktop: '桌面',
    device: '设备',
    tools: '工具',
    settings: '设置',
  };
  return labels[page];
}
