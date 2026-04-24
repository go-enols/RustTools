import { X, AlertCircle } from "lucide-react";
import { useEffect } from "react";

export interface AppError {
  id: string;
  message: string;
  detail?: string;
  type?: "error" | "warning" | "info";
}

interface ErrorToastProps {
  errors: AppError[];
  onDismiss: (id: string) => void;
}

export function ErrorToast({ errors, onDismiss }: ErrorToastProps) {
  return (
    <div className="fixed top-4 right-4 z-[9999] flex flex-col gap-2 w-80">
      {errors.map((err) => (
        <ErrorItem key={err.id} error={err} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ErrorItem({
  error,
  onDismiss,
}: {
  error: AppError;
  onDismiss: (id: string) => void;
}) {
  useEffect(() => {
    const t = setTimeout(() => onDismiss(error.id), 8000);
    return () => clearTimeout(t);
  }, [error.id, onDismiss]);

  const bg =
    error.type === "warning"
      ? "bg-amber-50 dark:bg-amber-950 border-amber-200 dark:border-amber-800"
      : error.type === "info"
      ? "bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800"
      : "bg-red-50 dark:bg-red-950 border-red-200 dark:border-red-800";

  const iconColor =
    error.type === "warning"
      ? "text-amber-500"
      : error.type === "info"
      ? "text-blue-500"
      : "text-red-500";

  return (
    <div
      className={`rounded-xl border shadow-lg p-3 flex gap-2 items-start animate-in slide-in-from-right ${bg}`}
    >
      <AlertCircle className={`w-5 h-5 shrink-0 mt-0.5 ${iconColor}`} />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-gray-900 dark:text-white">
          {error.message}
        </p>
        {error.detail && (
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 break-words">
            {error.detail}
          </p>
        )}
      </div>
      <button
        onClick={() => onDismiss(error.id)}
        className="shrink-0 p-0.5 rounded hover:bg-black/5 dark:hover:bg-white/10"
      >
        <X className="w-4 h-4 text-gray-400" />
      </button>
    </div>
  );
}
