import { useState } from 'react';
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  File,
  Plus,
  RefreshCw,
  Home,
} from 'lucide-react';
import { PageType } from '../../../../core/stores/routerStore';

interface SidebarProps {
  currentPage: PageType;
  activeSidebar: 'explorer' | 'search' | 'none';
  onNewProject: () => void;
  onGoHome: () => void;
}

// Mock file tree structure
interface FileNode {
  name: string;
  type: 'file' | 'folder';
  children?: FileNode[];
  expanded?: boolean;
}

const mockFileTree: FileNode[] = [
  {
    name: 'images',
    type: 'folder',
    expanded: true,
    children: [
      { name: 'train', type: 'folder', children: [{ name: 'img001.jpg', type: 'file' }] },
      { name: 'val', type: 'folder', children: [{ name: 'img002.jpg', type: 'file' }] },
    ],
  },
  {
    name: 'labels',
    type: 'folder',
    expanded: true,
    children: [
      { name: 'train', type: 'folder', children: [{ name: 'img001.txt', type: 'file' }] },
      { name: 'val', type: 'folder', children: [{ name: 'img002.txt', type: 'file' }] },
    ],
  },
  { name: 'models', type: 'folder', children: [] },
  { name: 'data.yaml', type: 'file' },
  { name: 'train.py', type: 'file' },
];

export default function Sidebar({ currentPage, activeSidebar, onNewProject, onGoHome }: SidebarProps) {
  const [fileTree, setFileTree] = useState<FileNode[]>(mockFileTree);

  if (activeSidebar === 'none') {
    return null;
  }

  const toggleFolder = (path: number[]) => {
    setFileTree((prev) => {
      const newTree = [...prev];
      let current = newTree;
      for (let i = 0; i < path.length - 1; i++) {
        current = current[path[i]].children!;
      }
      const lastIndex = path[path.length - 1];
      current[lastIndex].expanded = !current[lastIndex].expanded;
      return newTree;
    });
  };

  const renderNode = (node: FileNode, path: number[], depth: number = 0) => {
    const isFolder = node.type === 'folder';
    const Icon = isFolder ? (node.expanded ? FolderOpen : Folder) : File;

    return (
      <div key={path.join('-')} className="file-tree-node">
        <div
          className="file-tree-item"
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          onClick={() => isFolder && toggleFolder(path)}
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

  return (
    <aside className="sidebar sidebar-explorer">
      <div className="sidebar-header">
        <span>资源管理器</span>
        <div className="sidebar-actions">
          <button className="sidebar-action-btn" onClick={onGoHome} title="返回首页">
            <Home size={16} />
          </button>
          <button className="sidebar-action-btn" onClick={onNewProject} title="新建项目">
            <Plus size={16} />
          </button>
          <button className="sidebar-action-btn" onClick={() => {}} title="刷新">
            <RefreshCw size={16} />
          </button>
        </div>
      </div>

      <div className="sidebar-content">
        <div className="explorer-section">
          <div className="explorer-section-header">
            <ChevronDown size={14} />
            <span>项目名称</span>
          </div>
          <div className="file-tree">
            {fileTree.map((node, index) => renderNode(node, [index]))}
          </div>
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
    device: '设备',
    tools: '工具',
    settings: '设置',
  };
  return labels[page];
}
