import { useState } from 'react';

type HelpType = 'shortcuts' | 'docs' | 'update' | 'about';

interface TopMenuProps {
  onNewProject: () => void;
  onShowHelp: (type: HelpType) => void;
}

const menuItems = [
  { label: '文件', items: ['新建项目 (Ctrl+N)', '打开项目 (Ctrl+O)', '保存 (Ctrl+S)', '退出'] },
  { label: '项目', items: ['项目设置', '导入数据集', '导出模型', '转换模型'] },
  { label: '编辑', items: ['撤销', '重做', '复制', '粘贴'] },
  { label: '格式', items: ['图像大小', '标注格式', '预处理'] },
  { label: '工具', items: ['GPU设置', '训练日志', '性能分析'] },
  { label: '帮助', items: ['使用文档', '快捷键', '检查更新', '关于'] },
];

export default function TopMenu({ onNewProject, onShowHelp }: TopMenuProps) {
  const [activeMenu, setActiveMenu] = useState<string | null>(null);

  const handleMenuItemClick = (item: string) => {
    setActiveMenu(null);
    if (item.startsWith('新建项目')) {
      onNewProject();
    } else if (item === '快捷键') {
      onShowHelp('shortcuts');
    } else if (item === '使用文档') {
      onShowHelp('docs');
    } else if (item === '检查更新') {
      onShowHelp('update');
    } else if (item === '关于') {
      onShowHelp('about');
    }
  };

  return (
    <header className="top-menu">
      {menuItems.map((menu) => (
        <div
          key={menu.label}
          className={`top-menu-item ${activeMenu === menu.label ? 'active' : ''}`}
          onMouseEnter={() => setActiveMenu(menu.label)}
          onMouseLeave={() => setActiveMenu(null)}
        >
          {menu.label}
          {activeMenu === menu.label && (
            <div
              style={{
                position: 'absolute',
                top: '100%',
                left: 0,
                minWidth: 180,
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border-default)',
                borderRadius: 'var(--radius-md)',
                padding: 'var(--spacing-xs)',
                zIndex: 100,
                boxShadow: 'var(--shadow-lg)',
              }}
            >
              {menu.items.map((item, i) => (
                <div
                  key={i}
                  style={{
                    padding: 'var(--spacing-sm) var(--spacing-md)',
                    borderRadius: 'var(--radius-sm)',
                    cursor: 'pointer',
                    fontSize: 13,
                    color: 'var(--text-secondary)',
                  }}
                  onClick={() => handleMenuItemClick(item)}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'var(--bg-hover)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.color = 'var(--text-secondary)';
                  }}
                >
                  {item}
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </header>
  );
}
