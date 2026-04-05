import { useEffect, useState } from 'react';
import { useToastStore, ToastType } from '../../stores/toastStore';

interface ToastProps {
  id: string;
  message: string;
  type: ToastType;
  duration: number;
  onClose: () => void;
}

const typeStyles: Record<ToastType, { bg: string; border: string; icon: string }> = {
  info: {
    bg: 'var(--toast-info-bg)',
    border: 'var(--toast-info-border)',
    icon: 'ℹ️',
  },
  success: {
    bg: 'var(--toast-success-bg)',
    border: 'var(--toast-success-border)',
    icon: '✓',
  },
  warning: {
    bg: 'var(--toast-warning-bg)',
    border: 'var(--toast-warning-border)',
    icon: '⚠',
  },
  error: {
    bg: 'var(--toast-error-bg)',
    border: 'var(--toast-error-border)',
    icon: '✕',
  },
};

export default function Toast({ id, message, type, duration, onClose }: ToastProps) {
  const [remaining, setRemaining] = useState(duration);
  const [isHovered, setIsHovered] = useState(false);
  const removeToast = useToastStore((state) => state.removeToast);
  const style = typeStyles[type];

  useEffect(() => {
    if (isHovered) return; // Pause timer on hover

    const interval = 50; // Update every 50ms for smooth progress
    const timer = setInterval(() => {
      setRemaining((prev) => {
        const next = prev - interval;
        if (next <= 0) {
          clearInterval(timer);
          removeToast(id);
          return 0;
        }
        return next;
      });
    }, interval);

    return () => clearInterval(timer);
  }, [id, isHovered, removeToast]);

  const progress = (remaining / duration) * 100;

  return (
    <div
      style={{
        position: 'relative',
        minWidth: 320,
        maxWidth: 420,
        padding: '12px 40px 12px 12px',
        background: style.bg,
        borderRadius: 8,
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
        color: 'var(--text-primary)',
        fontSize: 14,
        lineHeight: 1.4,
        overflow: 'hidden',
        animation: 'slideIn 0.3s ease-out',
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Icon and message */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
        <span
          style={{
            width: 20,
            height: 20,
            borderRadius: '50%',
            background: 'rgba(255,255,255,0.2)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 12,
            flexShrink: 0,
          }}
        >
          {style.icon}
        </span>
        <span style={{ wordBreak: 'break-word' }}>{message}</span>
      </div>

      {/* Close button */}
      <button
        onClick={onClose}
        style={{
          position: 'absolute',
          top: 8,
          right: 8,
          width: 20,
          height: 20,
          border: 'none',
          background: 'rgba(255,255,255,0.2)',
          borderRadius: 4,
          color: 'var(--text-primary)',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: 12,
          padding: 0,
          lineHeight: 1,
        }}
      >
        ✕
      </button>

      {/* Progress bar */}
      <div
        style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          height: 3,
          width: `${progress}%`,
          background: style.border,
          transition: 'width 0.05s linear',
        }}
      />

      <style>{`
        @keyframes slideIn {
          from {
            transform: translateX(100%);
            opacity: 0;
          }
          to {
            transform: translateX(0);
            opacity: 1;
          }
        }
      `}</style>
    </div>
  );
}
