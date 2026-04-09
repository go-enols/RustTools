#!/usr/bin/env python3
"""
YOLO Video Inference Script
Receives video path + model path, runs inference, emits JSON events to stdout.
"""

import sys
import json
import argparse
import warnings
from pathlib import Path
from datetime import datetime

warnings.filterwarnings("ignore")

try:
    from ultralytics import YOLO
except ImportError:
    print(json.dumps({"type": "error", "error": "ultralytics not installed. Run: pip install ultralytics"}), flush=True)
    sys.exit(1)


def send_event(event_type: str, data: dict):
    print(json.dumps({"type": event_type, **data}), flush=True)


def format_boxes(results) -> list:
    """Convert YOLO results to annotation boxes."""
    boxes = []
    for result in results:
        if result.boxes is None:
            continue
        for box in result.boxes:
            xyxy = box.xyxy[0].cpu().numpy()
            conf = float(box.conf[0].cpu().numpy())
            cls = int(box.cls[0].cpu().numpy())
            cls_name = result.names[cls]
            boxes.append({
                "class_id": cls,
                "class_name": cls_name,
                "confidence": conf,
                "x1": float(xyxy[0]),
                "y1": float(xyxy[1]),
                "x2": float(xyxy[2]),
                "y2": float(xyxy[3]),
            })
    return boxes


def main():
    parser = argparse.ArgumentParser(description="YOLO Video Inference")
    parser.add_argument("video_path", help="Path to input video")
    parser.add_argument("model_path", help="Path to YOLO model (.pt file)")
    parser.add_argument("--conf", type=float, default=0.65, help="Confidence threshold")
    parser.add_argument("--iou", type=float, default=0.5, help="IoU threshold for NMS")
    parser.add_argument("--device", type=str, default="0", help="Device (0=cuda, cpu)")
    parser.add_argument("--output-json", type=str, default="", help="Output JSON file path")
    parser.add_argument("--frame-interval", type=int, default=1, help="Process every N frames")
    args = parser.parse_args()

    video_path = Path(args.video_path)
    if not video_path.exists():
        send_event("error", {"error": f"Video file not found: {video_path}"})
        sys.exit(1)

    model_path = Path(args.model_path)
    if not model_path.exists():
        send_event("error", {"error": f"Model file not found: {model_path}"})
        sys.exit(1)

    send_event("status", {"message": f"Loading model from {model_path}..."})
    try:
        model = YOLO(str(model_path))
        send_event("status", {"message": "Model loaded successfully"})
    except Exception as e:
        send_event("error", {"error": f"Failed to load model: {e}"})
        sys.exit(1)

    send_event("status", {"message": f"Opening video {video_path}..."})
    try:
        import cv2
        cap = cv2.VideoCapture(str(video_path))
        if not cap.isOpened():
            send_event("error", {"error": "Failed to open video"})
            sys.exit(1)

        fps = cap.get(cv2.CAP_PROP_FPS)
        frame_count = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
        width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
        height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
        send_event("metadata", {
            "fps": fps,
            "frame_count": frame_count,
            "width": width,
            "height": height,
        })
    except Exception as e:
        send_event("error", {"error": f"Failed to open video: {e}"})
        sys.exit(1)

    send_event("status", {"message": f"Starting inference: {frame_count} frames @ {fps:.1f}fps"})
    
    all_results = []
    frame_idx = 0
    processed_idx = 0

    while True:
        ret, frame = cap.read()
        if not ret:
            break

        timestamp_ms = int((frame_idx / fps) * 1000)

        # Process every N frames
        if frame_idx % args.frame_interval == 0:
            try:
                results = model.predict(
                    frame,
                    conf=args.conf,
                    iou=args.iou,
                    device=args.device,
                    verbose=False,
                    stream=True,
                )
                boxes = format_boxes(results)

                send_event("progress", {
                    "frame": frame_idx,
                    "total_frames": frame_count,
                    "timestamp_ms": timestamp_ms,
                    "boxes": boxes,
                    "num_detections": len(boxes),
                })

                all_results.append({
                    "frame_index": frame_idx,
                    "timestamp_ms": timestamp_ms,
                    "boxes": boxes,
                })
                processed_idx += 1
            except Exception as e:
                send_event("error", {"error": f"Inference error at frame {frame_idx}: {e}"})

        frame_idx += 1

        if frame_idx % 100 == 0:
            send_event("status", {
                "message": f"Processed {frame_idx}/{frame_count} frames ({processed_idx} inference frames)"
            })

    cap.release()
    send_event("status", {"message": f"Inference complete: {processed_idx} frames processed"})
    send_event("complete", {"total_frames": frame_idx, "inference_frames": processed_idx})

    # Save results to JSON if requested
    if args.output_json:
        try:
            with open(args.output_json, "w", encoding="utf-8") as f:
                json.dump(all_results, f, ensure_ascii=False, indent=2)
            send_event("status", {"message": f"Results saved to {args.output_json}"})
        except Exception as e:
            send_event("error", {"error": f"Failed to save results: {e}"})


if __name__ == "__main__":
    main()
