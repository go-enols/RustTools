import { useCallback } from 'react';
import { useToastStore, ToastType } from '../stores/toastStore';

export function useToast() {
  const addToast = useToastStore((state) => state.addToast);

  const toast = useCallback(
    (message: string, type: ToastType = 'info', duration: number = 5000) => {
      addToast(message, type, duration);
    },
    [addToast]
  );

  const info = useCallback(
    (message: string, duration?: number) => toast(message, 'info', duration),
    [toast]
  );

  const success = useCallback(
    (message: string, duration?: number) => toast(message, 'success', duration),
    [toast]
  );

  const warning = useCallback(
    (message: string, duration?: number) => toast(message, 'warning', duration),
    [toast]
  );

  const error = useCallback(
    (message: string, duration?: number) => toast(message, 'error', duration),
    [toast]
  );

  return { toast, info, success, warning, error };
}
