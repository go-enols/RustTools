import { useState, useEffect, ReactNode } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
}

export function Modal({ isOpen, onClose, title, children, footer }: ModalProps) {
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = '';
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3 className="modal-title">{title}</h3>
          <button className="modal-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-footer">{footer}</div>}
      </div>
      <style>{`
        .modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.6);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 2000;
          animation: modalFadeIn 0.15s ease-out;
        }
        .modal-content {
          background: var(--bg-elevated);
          border: 1px solid var(--border-default);
          border-radius: 12px;
          min-width: 360px;
          max-width: 480px;
          box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
          animation: modalSlideIn 0.15s ease-out;
        }
        .modal-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 16px 20px;
          border-bottom: 1px solid var(--border-default);
        }
        .modal-title {
          margin: 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary);
        }
        .modal-close {
          background: none;
          border: none;
          padding: 4px;
          cursor: pointer;
          color: var(--text-secondary);
          border-radius: 4px;
          display: flex;
          align-items: center;
          justify-content: center;
          transition: all 0.15s;
        }
        .modal-close:hover {
          background: var(--bg-hover);
          color: var(--text-primary);
        }
        .modal-body {
          padding: 20px;
        }
        .modal-footer {
          display: flex;
          justify-content: flex-end;
          gap: 12px;
          padding: 16px 20px;
          border-top: 1px solid var(--border-default);
        }
        @keyframes modalFadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes modalSlideIn {
          from { opacity: 0; transform: scale(0.95) translateY(-10px); }
          to { opacity: 1; transform: scale(1) translateY(0); }
        }
      `}</style>
    </div>
  );
}

interface InputModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (value: string) => void;
  title: string;
  label: string;
  placeholder?: string;
  defaultValue?: string;
}

export function InputModal({ isOpen, onClose, onConfirm, title, label, placeholder, defaultValue }: InputModalProps) {
  const [value, setValue] = useState(defaultValue || '');

  useEffect(() => {
    if (isOpen) setValue(defaultValue || '');
  }, [isOpen, defaultValue]);

  const handleConfirm = () => {
    if (value.trim()) {
      onConfirm(value.trim());
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={title}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onClose}>取消</button>
          <button className="btn btn-primary" onClick={handleConfirm} disabled={!value.trim()}>确定</button>
        </>
      }
    >
      <div className="input-modal-body">
        <label className="input-label">{label}</label>
        <input
          type="text"
          className="input-field"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder}
          autoFocus
          onKeyDown={(e) => {
            if (e.key === 'Enter' && value.trim()) handleConfirm();
          }}
        />
      </div>
      <style>{`
        .input-modal-body {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .input-label {
          font-size: 13px;
          color: var(--text-secondary);
        }
        .input-field {
          width: 100%;
          padding: 10px 12px;
          font-size: 14px;
          color: var(--text-primary);
          background: var(--bg-input);
          border: 1px solid var(--border-default);
          border-radius: 6px;
          outline: none;
          transition: border-color 0.15s;
          box-sizing: border-box;
        }
        .input-field:focus {
          border-color: var(--accent-primary);
        }
        .input-field::placeholder {
          color: var(--text-tertiary);
        }
        .btn {
          padding: 8px 16px;
          font-size: 13px;
          font-weight: 500;
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.15s;
          border: none;
        }
        .btn-primary {
          background: var(--accent-primary);
          color: var(--text-primary);
        }
        .btn-primary:hover:not(:disabled) {
          background: var(--accent-hover);
        }
        .btn-primary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .btn-secondary {
          background: var(--bg-hover);
          color: var(--text-primary);
          border: 1px solid var(--border-default);
        }
        .btn-secondary:hover {
          background: var(--bg-active);
        }
      `}</style>
    </Modal>
  );
}

interface ConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'danger' | 'warning' | 'info';
}

export function ConfirmModal({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  confirmText = '确定',
  cancelText = '取消',
  variant = 'danger',
}: ConfirmModalProps) {
  const variantStyles = {
    danger: { bg: 'var(--status-error)', hover: 'var(--accent-hover)' },
    warning: { bg: 'var(--status-warning)', hover: 'var(--accent-hover)' },
    info: { bg: 'var(--status-info)', hover: 'var(--accent-hover)' },
  };

  const style = variantStyles[variant];

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={title}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onClose}>{cancelText}</button>
          <button className="btn btn-danger" onClick={() => { onConfirm(); onClose(); }} style={{ background: style.bg }}>{confirmText}</button>
        </>
      }
    >
      <p className="confirm-message">{message}</p>
      <style>{`
        .confirm-message {
          margin: 0;
          font-size: 14px;
          color: var(--text-primary);
          line-height: 1.5;
        }
        .btn-danger:hover {
          background: var(--accent-hover) !important;
        }
      `}</style>
    </Modal>
  );
}
