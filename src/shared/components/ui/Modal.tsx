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

interface DownloadModalProps {
  isOpen: boolean;
  onClose?: () => void;
  title: string;
  message?: string;
  progress?: string;
  error?: string;
  onRetry?: () => void;
}

export function DownloadModal({
  isOpen,
  onClose,
  title,
  message,
  progress,
  error,
  onRetry,
}: DownloadModalProps) {
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className="download-modal-overlay">
      <div className="download-modal-content">
        <div className="download-modal-header">
          <h3 className="download-modal-title">{title}</h3>
          <button
            className="download-modal-close"
            onClick={onClose}
            title="关闭"
          >
            ✕
          </button>
        </div>
        <div className="download-modal-body">
          {message && <p className="download-message">{message}</p>}

          {progress && !error && (
            <div className="download-progress-section">
              <div className="download-spinner-wrapper">
                <div className="download-spinner" />
                <div className="download-spinner-text">正在下载...</div>
              </div>
              <div className="download-progress-text">{progress}</div>
            </div>
          )}

          {error && (
            <div className="download-error-section">
              <div className="download-error-icon">⚠️</div>
              <div className="download-error-content">
                <p className="download-error-title">下载失败</p>
                <p className="download-error-message">{error}</p>
              </div>
              <div className="download-error-actions">
                {onRetry && (
                  <button
                    className="download-btn download-btn-retry"
                    onClick={onRetry}
                  >
                    🔄 重试
                  </button>
                )}
                {onClose && (
                  <button
                    className="download-btn download-btn-close"
                    onClick={onClose}
                  >
                    关闭
                  </button>
                )}
              </div>
            </div>
          )}

          {!error && !progress && message && (
            <div className="download-waiting">
              <div className="download-spinner-wrapper">
                <div className="download-spinner" />
                <div className="download-spinner-text">准备中...</div>
              </div>
            </div>
          )}
        </div>
      </div>
      <style>{`
        .download-modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.75);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 2000;
          animation: modalFadeIn 0.2s ease-out;
        }
        .download-modal-content {
          background: var(--bg-elevated);
          border: 1px solid var(--border-default);
          border-radius: 16px;
          width: 420px;
          max-width: 90vw;
          box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
          animation: modalSlideIn 0.2s ease-out;
          overflow: hidden;
        }
        .download-modal-header {
          padding: 20px 24px;
          border-bottom: 1px solid var(--border-subtle);
          background: linear-gradient(135deg, var(--bg-elevated) 0%, var(--bg-secondary) 100%);
        }
        .download-modal-title-wrapper {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .download-modal-title {
          margin: 0;
          font-size: 18px;
          font-weight: 600;
          color: var(--text-primary);
        }
        .download-modal-close {
          background: none;
          border: none;
          padding: 8px;
          cursor: pointer;
          color: var(--text-tertiary);
          font-size: 20px;
          line-height: 1;
          border-radius: 6px;
          transition: all 0.15s;
        }
        .download-modal-close:hover {
          background: var(--bg-hover);
          color: var(--text-primary);
        }
        .download-modal-body {
          padding: 24px;
        }
        .download-message {
          margin: 0 0 20px 0;
          font-size: 14px;
          color: var(--text-secondary);
          line-height: 1.6;
        }
        .download-progress-section {
          text-align: center;
          padding: 20px 0;
        }
        .download-spinner-wrapper {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 12px;
          margin-bottom: 16px;
        }
        .download-spinner {
          width: 48px;
          height: 48px;
          border: 4px solid var(--border-default);
          border-top-color: var(--accent-primary);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }
        .download-spinner-text {
          font-size: 14px;
          color: var(--text-tertiary);
          font-weight: 500;
        }
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
        .download-progress-text {
          font-size: 14px;
          color: var(--text-secondary);
          margin-top: 12px;
        }
        .download-error-section {
          text-align: center;
          padding: 20px 0;
        }
        .download-error-icon {
          font-size: 48px;
          margin-bottom: 16px;
        }
        .download-error-content {
          margin-bottom: 20px;
        }
        .download-error-title {
          margin: 0 0 8px 0;
          font-size: 18px;
          font-weight: 600;
          color: var(--status-error, #ff4d4f);
        }
        .download-error-message {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary);
          line-height: 1.5;
        }
        .download-error-actions {
          display: flex;
          flex-direction: column;
          gap: 10px;
          margin-top: 20px;
        }
        .download-btn {
          padding: 12px 24px;
          font-size: 14px;
          font-weight: 500;
          border-radius: 8px;
          cursor: pointer;
          transition: all 0.15s;
          border: none;
          width: 100%;
        }
        .download-btn-retry {
          background: linear-gradient(135deg, var(--accent-primary) 0%, var(--accent-secondary) 100%);
          color: white;
        }
        .download-btn-retry:hover {
          transform: translateY(-1px);
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
        }
        .download-btn-secondary {
          background: var(--bg-hover);
          color: var(--text-primary);
          border: 1px solid var(--border-default);
        }
        .download-btn-secondary:hover {
          background: var(--bg-active);
        }
        .download-btn-close {
          background: transparent;
          color: var(--text-tertiary);
          border: 1px solid var(--border-subtle);
        }
        .download-btn-close:hover {
          background: var(--bg-hover);
          color: var(--text-secondary);
        }
        .download-waiting {
          text-align: center;
          padding: 40px 0;
        }
        @keyframes modalFadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes modalSlideIn {
          from { opacity: 0; transform: scale(0.95) translateY(-20px); }
          to { opacity: 1; transform: scale(1) translateY(0); }
        }
      `}</style>
    </div>
  );
}
