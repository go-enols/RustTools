import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

export function usePtExport() {
  const [needsExport, setNeedsExport] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState("");
  const [exportSuccess, setExportSuccess] = useState(false);
  const successTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const checkModel = useCallback(async (path: string) => {
    setExportError("");
    setExportSuccess(false);
    if (successTimerRef.current) {
      clearTimeout(successTimerRef.current);
      successTimerRef.current = null;
    }
    if (path.toLowerCase().endsWith(".pt")) {
      const hasOnnx = await invoke<boolean>("check_onnx_for_pt", { modelPath: path });
      setNeedsExport(!hasOnnx);
    } else {
      setNeedsExport(false);
    }
  }, []);

  const doExport = useCallback(async (path: string) => {
    setExporting(true);
    setExportError("");
    setExportSuccess(false);
    if (successTimerRef.current) {
      clearTimeout(successTimerRef.current);
      successTimerRef.current = null;
    }
    try {
      const onnxPath = await invoke<string>("export_pt_to_onnx", { modelPath: path });
      setNeedsExport(false);
      setExportSuccess(true);
      successTimerRef.current = setTimeout(() => setExportSuccess(false), 4000);
      return onnxPath;
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e.message || "转换失败";
      setExportError(msg);
      throw e;
    } finally {
      setExporting(false);
    }
  }, []);

  return { needsExport, exporting, exportError, exportSuccess, checkModel, doExport };
}
