#!/usr/bin/env python3
"""
将 Ultralytics YOLO .pt 模型导出为 ONNX 格式。
Usage: python export_pt_to_onnx.py <model_path> [imgsz]
Output: 导出的 ONNX 文件绝对路径（stdout 最后一行）
"""
import sys
import os


def main():
    if len(sys.argv) < 2:
        print("Usage: export_pt_to_onnx.py <model_path> [imgsz]", file=sys.stderr)
        sys.exit(1)

    model_path = sys.argv[1]
    imgsz = int(sys.argv[2]) if len(sys.argv) > 2 else 640

    if not os.path.exists(model_path):
        print(f"Model not found: {model_path}", file=sys.stderr)
        sys.exit(1)

    try:
        from ultralytics import YOLO
    except ImportError as e:
        print(f"ultralytics not installed: {e}", file=sys.stderr)
        sys.exit(1)

    model = YOLO(model_path)
    # export 会生成同名 .onnx 文件
    model.export(format="onnx", imgsz=imgsz, simplify=True, opset=12)

    onnx_path = os.path.splitext(model_path)[0] + ".onnx"
    if os.path.exists(onnx_path):
        print(os.path.abspath(onnx_path))
    else:
        # 某些情况下 export 可能生成不同名称，尝试查找
        base_dir = os.path.dirname(model_path) or "."
        base_name = os.path.splitext(os.path.basename(model_path))[0]
        candidates = [
            os.path.join(base_dir, base_name + ".onnx"),
            os.path.join(base_dir, base_name + "_cpu.onnx"),
        ]
        for c in candidates:
            if os.path.exists(c):
                print(os.path.abspath(c))
                return
        print(f"ONNX export failed: expected {onnx_path} not found", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
