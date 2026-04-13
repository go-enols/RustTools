import { X, Keyboard, FileText, RefreshCw, Info } from 'lucide-react';

export type HelpType = 'shortcuts' | 'docs' | 'update' | 'about';

interface ShortcutItem {
  keys: string[];
  description: string;
}

interface HelpModalProps {
  type: HelpType;
  onClose: () => void;
}

const shortcutData: ShortcutItem[] = [
  { keys: ['Ctrl', 'N'], description: '新建项目' },
  { keys: ['Ctrl', 'O'], description: '打开项目' },
  { keys: ['Ctrl', 'S'], description: '保存项目' },
  { keys: ['F1'], description: '使用帮助' },
  { keys: ['Q'], description: '选择/拖动模式 (标注页)' },
  { keys: ['W'], description: '绘制模式 (标注页)' },
  { keys: ['A'], description: '上一张图片 (标注页)' },
  { keys: ['D'], description: '下一张图片 (标注页)' },
  { keys: ['Delete'], description: '删除选中标注 (标注页)' },
  { keys: ['Esc'], description: '取消选择 (标注页)' },
];

const contentMap = {
  shortcuts: {
    title: '键盘快捷键',
    icon: Keyboard,
    sections: [
      { title: '全局快捷键', items: shortcutData.slice(0, 4) },
      { title: '标注页面快捷键', items: shortcutData.slice(4) },
    ],
  },
  docs: {
    title: '使用文档',
    icon: FileText,
    paragraphs: [
      'MyRustTools 是一款基于 Rust 编写的工具集合应用。',
      '主要功能：',
      '• Yolo 数据集标注、模型训练、模型测试',
      '',
      '详细文档正在完善中...',
    ],
  },
  update: {
    title: '检查更新',
    icon: RefreshCw,
    paragraphs: [
      'MyRustTools',
      '',
      '当前版本：v1.0.0',
      '构建日期：2026-04-04',
      '',
      '已是最新版本，无需更新。',
    ],
  },
  about: {
    title: '关于 MyRustTools',
    icon: Info,
    paragraphs: [
      'MyRustTools v1.0.0',
      '',
      '基于 YOLO 的目标检测标注和训练工具。',
      '',
      '支持 YOLOv5、YOLOv8、YOLO11 等多种模型格式。',
      '',
      '© 2026 MyRustTools',
    ],
  },
};

export default function HelpModal({ type, onClose }: HelpModalProps) {
  const content = contentMap[type];
  const Icon = content.icon;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 520 }}>
        <div className="modal-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
            <Icon size={18} style={{ color: 'var(--accent-primary)' }} />
            <h2 className="modal-title">{content.title}</h2>
          </div>
          <button className="btn btn-ghost" style={{ padding: 4 }} onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {'sections' in content ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-xl)' }}>
              {content.sections.map((section) => (
                <div key={section.title}>
                  <h4
                    style={{
                      fontSize: 13,
                      color: 'var(--text-secondary)',
                      marginBottom: 'var(--spacing-md)',
                      fontWeight: 500,
                    }}
                  >
                    {section.title}
                  </h4>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                    {section.items.map((item, i) => (
                      <div
                        key={i}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          padding: 'var(--spacing-sm) var(--spacing-md)',
                          background: 'var(--bg-elevated)',
                          borderRadius: 'var(--radius-md)',
                        }}
                      >
                        <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{item.description}</span>
                        <div style={{ display: 'flex', gap: 4 }}>
                          {item.keys.map((key) => (
                            <kbd
                              key={key}
                              style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                minWidth: 24,
                                height: 24,
                                padding: '0 8px',
                                fontSize: 11,
                                fontFamily: 'monospace',
                                fontWeight: 500,
                                color: 'var(--text-primary)',
                                background: 'var(--bg-surface)',
                                border: '1px solid var(--border-default)',
                                borderRadius: 'var(--radius-sm)',
                                boxShadow: 'var(--shadow-kbd)',
                              }}
                            >
                              {key}
                            </kbd>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
              {content.paragraphs.map((p, i) => (
                <p
                  key={i}
                  style={{
                    fontSize: 13,
                    color: p === '' ? 'inherit' : 'var(--text-secondary)',
                    lineHeight: 1.6,
                    margin: p === '' ? 'var(--spacing-md)' : 0,
                  }}
                >
                  {p}
                </p>
              ))}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-primary" onClick={onClose}>
            确定
          </button>
        </div>
      </div>
    </div>
  );
}
