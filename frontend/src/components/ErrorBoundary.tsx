import React from "react";
import { AlertCircle } from "lucide-react";

interface Props {
  children: React.ReactNode;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    this.props.onError?.(error, errorInfo);
    console.error("ErrorBoundary caught:", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen flex items-center justify-center bg-bg dark:bg-bg-dark p-6">
          <div className="max-w-md w-full bg-surface dark:bg-surface-dark rounded-2xl border border-gray-200 dark:border-gray-800 shadow-xl p-8 text-center">
            <AlertCircle className="w-12 h-12 text-brand-danger mx-auto mb-4" />
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              出错了
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              应用遇到了意外错误，请刷新页面重试。
            </p>
            {this.state.error && (
              <pre className="text-xs bg-gray-100 dark:bg-gray-900 rounded-lg p-3 text-left overflow-auto max-h-40 text-gray-700 dark:text-gray-300">
                {this.state.error.toString()}
              </pre>
            )}
            <button
              onClick={() => window.location.reload()}
              className="mt-6 px-4 py-2 bg-brand-primary text-white rounded-lg text-sm font-medium hover:bg-blue-600 transition"
            >
              刷新页面
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
