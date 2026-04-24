import { createContext, useContext } from "react";

export interface AppError {
  id: string;
  message: string;
  detail?: string;
  type?: "error" | "warning" | "info";
}

interface AppErrorContextValue {
  addError: (err: Omit<AppError, "id">) => void;
  dismissError: (id: string) => void;
}

export const AppErrorContext = createContext<AppErrorContextValue | null>(null);

export function useAppError() {
  const ctx = useContext(AppErrorContext);
  if (!ctx) {
    throw new Error("useAppError must be used within AppErrorContext.Provider");
  }
  return ctx;
}
