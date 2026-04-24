#!/usr/bin/env python3
"""
YOLO Inference script for auto-annotation.
Usage: python yolo_inference.py <model_path> <image_path> [conf_threshold]
Output: JSON array of detections to stdout (last line)
"""
import json
import sys
from pathlib import Path


def run_inference(model_path: str, image_path: str, conf_threshold: float = 0.25):
    try:
        from ultralytics import YOLO

        model = YOLO(model_path)
        results = model(image_path, conf=conf_threshold, verbose=False)

        detections = []
        for result in results:
            boxes = result.boxes
            if boxes is None:
                continue
            for box in boxes:
                cls = int(box.cls.item()) if hasattr(box.cls, "item") else int(box.cls)
                conf = float(box.conf.item()) if hasattr(box.conf, "item") else float(box.conf)
                xyxy = box.xyxy[0].tolist() if hasattr(box.xyxy, "tolist") else list(box.xyxy[0])
                x1, y1, x2, y2 = xyxy
                detections.append(
                    {
                        "class_id": cls,
                        "confidence": conf,
                        "x1": float(x1),
                        "y1": float(y1),
                        "x2": float(x2),
                        "y2": float(y2),
                    }
                )
        return detections
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(
            "Usage: yolo_inference.py <model_path> <image_path> [conf_threshold]",
            file=sys.stderr,
        )
        sys.exit(1)

    model_path = sys.argv[1]
    image_path = sys.argv[2]
    conf = float(sys.argv[3]) if len(sys.argv) > 3 else 0.25

    detections = run_inference(model_path, image_path, conf)
    print(json.dumps(detections))
