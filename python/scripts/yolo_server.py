#!/usr/bin/env python3
"""
YOLO Training Sidecar for RustTools
Communicates via stdin/stdout JSON lines protocol.
"""

import json
import sys
import os
import threading
import traceback
import time
from pathlib import Path

# Global flag for stopping training
_stop_requested = False
_current_trainer = None


def send_event(event: dict):
    """Send a JSON event to stdout."""
    try:
        print(json.dumps(event), flush=True)
    except Exception as e:
        print(json.dumps({"type": "error", "error": f"Send event failed: {e}"}), flush=True)


def check_ultralytics():
    """Check if ultralytics is installed."""
    try:
        import ultralytics
        return True, ultralytics.__version__
    except ImportError:
        return False, None


def ensure_data_yaml(project_path: str, config: dict) -> str:
    """Ensure data.yaml exists for the project."""
    project = Path(project_path)
    data_yaml = project / "data.yaml"

    if data_yaml.exists():
        return str(data_yaml)

    # Try to find classes from project config or dataset
    classes = []

    # Look for data.yaml in parent directories
    for parent in [project, project.parent]:
        dy = parent / "data.yaml"
        if dy.exists():
            return str(dy)

    # Try to infer classes from dataset.yaml or labels
    dataset_yaml = project / "dataset.yaml"
    if dataset_yaml.exists():
        return str(dataset_yaml)

    # Create a basic data.yaml
    # Look for images directory
    images_dir = project / "images"
    train_dir = images_dir / "train"
    val_dir = images_dir / "val"

    if not train_dir.exists():
        train_dir = project / "train" / "images"
        val_dir = project / "val" / "images"

    if not train_dir.exists():
        # Fallback: use project path
        train_dir = project
        val_dir = project

    # Try to get class names
    classes_file = project / "classes.txt"
    if classes_file.exists():
        with open(classes_file, "r") as f:
            classes = [line.strip() for line in f if line.strip()]

    if not classes:
        # Default single class
        classes = ["object"]

    yaml_content = f"""path: {project_path}
train: images/train
val: images/val
nc: {len(classes)}
names: {classes}
"""
    data_yaml.write_text(yaml_content, encoding="utf-8")
    return str(data_yaml)


class TrainingCallback:
    """Callback to send training progress to Rust sidecar."""

    def __init__(self, total_epochs: int):
        self.total_epochs = total_epochs
        self.last_epoch = 0
        self.last_activity_time = time.time()
        self.callback_fired = {"pretrain": False, "epoch_start": False, "epoch_end": False}

    def _touch(self):
        self.last_activity_time = time.time()

    def on_pretrain_routine_start(self, trainer):
        self.callback_fired["pretrain"] = True
        self._touch()
        send_event({"type": "log", "data": {"message": "[Callback] Pretrain routine started"}})

    def on_train_epoch_start(self, trainer):
        self.callback_fired["epoch_start"] = True
        self._touch()
        epoch = trainer.epoch + 1
        send_event({"type": "log", "data": {"message": f"[Callback] Epoch {epoch}/{self.total_epochs} started"}})

    def on_train_epoch_end(self, trainer):
        global _stop_requested
        if _stop_requested:
            raise InterruptedError("Training stopped by user")

        epoch = trainer.epoch + 1
        self.last_epoch = epoch

        # Extract training losses from loss_items (list/tuple/tensor: [box, cls, dfl])
        loss_items = getattr(trainer, "loss_items", None)
        # Safely convert tensor -> list to avoid "Boolean value of Tensor..." error
        if loss_items is not None and hasattr(loss_items, "tolist"):
            try:
                loss_items = loss_items.tolist()
            except Exception:
                loss_items = None

        if isinstance(loss_items, (list, tuple)) and len(loss_items) >= 3:
            box_loss = float(loss_items[0]) if loss_items[0] is not None else 0.0
            cls_loss = float(loss_items[1]) if loss_items[1] is not None else 0.0
            dfl_loss = float(loss_items[2]) if loss_items[2] is not None else 0.0
        elif isinstance(loss_items, (list, tuple)) and len(loss_items) >= 2:
            box_loss = float(loss_items[0]) if loss_items[0] is not None else 0.0
            cls_loss = float(loss_items[1]) if loss_items[1] is not None else 0.0
            dfl_loss = 0.0
        else:
            box_loss = cls_loss = dfl_loss = 0.0

        # Extract validation metrics
        metrics = trainer.metrics or {}
        # Safely get metric value (may be tensor or float)
        def _get(m, *keys):
            for k in keys:
                if k in m:
                    v = m[k]
                    if hasattr(v, "item"):
                        try:
                            return v.item()
                        except Exception:
                            return float(v)
                    return float(v) if v is not None else 0.0
            return 0.0

        precision = _get(metrics, "metrics/precision(B)", "precision(B)", "precision")
        recall    = _get(metrics, "metrics/recall(B)",    "recall(B)",    "recall")
        map50     = _get(metrics, "metrics/mAP50(B)",     "mAP50(B)",     "mAP50")
        map50_95  = _get(metrics, "metrics/mAP50-95(B)",  "mAP50-95(B)",  "mAP50-95")

        # Send progress event
        event = {
            "type": "progress",
            "data": {
                "epoch": epoch,
                "total_epochs": self.total_epochs,
                "box_loss": box_loss,
                "cls_loss": cls_loss,
                "dfl_loss": dfl_loss,
                "precision": precision,
                "recall": recall,
                "mAP50": map50,
                "mAP50-95": map50_95,
            }
        }
        send_event(event)

        # Send log
        send_event({
            "type": "log",
            "data": {"message": f"Epoch {epoch}/{self.total_epochs} - box_loss: {box_loss:.4f}, cls_loss: {cls_loss:.4f}, dfl_loss: {dfl_loss:.4f}, mAP50: {map50:.4f}"}
        })

    def on_fit_epoch_end(self, trainer, *args):
        pass

    def on_train_end(self, trainer):
        send_event({"type": "log", "data": {"message": "Training finished"}})


def run_training(config: dict):
    """Run YOLO training with the given config."""
    global _stop_requested, _current_trainer
    _stop_requested = False

    project_path = config.get("project_path", ".")
    base_model = config.get("base_model", "yolo11n.pt")
    epochs = config.get("epochs", 100)
    batch_size = config.get("batch_size", 16)
    image_size = config.get("image_size", 640)
    device = config.get("device", "cpu")
    # YOLO sidecar mode: force workers=0 because DataLoader multiprocessing
    # conflicts with the sidecar stdin/stdout protocol and can deadlock.
    workers = 0
    optimizer = config.get("optimizer", "SGD")

    try:
        from ultralytics import YOLO
        import ultralytics

        # Disable OpenCV multithreading to avoid deadlocks with DataLoader workers=0
        try:
            import cv2
            cv2.setNumThreads(0)
            send_event({"type": "log", "data": {"message": "[Debug] OpenCV threads disabled"}})
        except Exception:
            pass

        # Limit PyTorch threads to avoid contention
        try:
            import torch
            torch.set_num_threads(1)
            send_event({"type": "log", "data": {"message": "[Debug] PyTorch threads limited to 1"}})
        except Exception:
            pass

        # Ensure data.yaml
        data_yaml = ensure_data_yaml(project_path, config)
        send_event({"type": "log", "data": {"message": f"Using dataset config: {data_yaml}"}})

        # Resolve model path — prefer absolute path from Rust, fallback to cache / CWD
        model_path = base_model
        if os.path.isfile(model_path):
            send_event({"type": "log", "data": {"message": f"Model file exists: {model_path}"}})
        else:
            # base_model may be a bare filename; search multiple locations
            candidates = [
                Path(project_path) / base_model,                         # project directory
                Path.home() / ".cache" / "ultralytics" / base_model,     # ultralytics cache
                Path.home() / ".rusttools" / "models" / base_model,      # RustTools cache
                Path(os.path.dirname(os.path.abspath(__file__))) / base_model, # script directory
            ]
            found = False
            for cand in candidates:
                if cand.exists():
                    model_path = str(cand.resolve())
                    send_event({"type": "log", "data": {"message": f"Resolved model: {model_path}"}})
                    found = True
                    break
            if not found:
                err_msg = f"Model not found: {base_model}. Searched: {', '.join(str(c) for c in candidates)}"
                send_event({"type": "error", "error": err_msg})
                return

        # Load model
        model = YOLO(model_path)
        _current_trainer = model

        send_event({"type": "started"})
        send_event({"type": "log", "data": {"message": f"Starting training: {epochs} epochs, batch={batch_size}, imgsz={image_size}"}})
        send_event({"type": "log", "data": {"message": f"Device: {device}, workers: {workers}, optimizer: {optimizer}"}})

        # Build training arguments
        train_args = {
            "data": data_yaml,
            "epochs": epochs,
            "batch": batch_size,
            "imgsz": image_size,
            "device": device if device != -1 else "cpu",
            "workers": workers,
            "optimizer": optimizer,
            "project": Path(project_path) / "runs",
            "name": "train",
            "exist_ok": True,
            "verbose": False,
        }

        # Optional arguments
        for key in ["lr0", "lrf", "momentum", "weight_decay", "warmup_epochs",
                    "warmup_bias_lr", "warmup_momentum", "hsv_h", "hsv_s", "hsv_v",
                    "translate", "scale", "shear", "perspective", "flipud", "fliplr",
                    "mosaic", "mixup", "copy_paste", "close_mosaic", "save_period"]:
            if key in config and config[key] is not None:
                train_args[key] = config[key]

        # Boolean flags
        for key in ["rect", "cos_lr", "single_cls", "amp", "cache"]:
            if key in config:
                train_args[key] = config[key]

        # Detect CUDA availability and fall back to CPU if needed
        current_device = train_args.get("device", "cpu")
        if current_device != "cpu" and current_device != -1:
            try:
                import torch
                if not torch.cuda.is_available():
                    send_event({"type": "log", "data": {"message": "CUDA not available, falling back to CPU"}})
                    train_args["device"] = "cpu"
                else:
                    # Quick CUDA sanity check
                    torch.cuda.synchronize()
                    send_event({"type": "log", "data": {"message": f"CUDA available: {torch.cuda.get_device_name(0)}"}})
            except Exception as e:
                send_event({"type": "log", "data": {"message": f"CUDA check failed: {e}, using CPU"}})
                train_args["device"] = "cpu"

        # Create callback
        callback = TrainingCallback(epochs)

        # Add callbacks to model
        model.add_callback("on_pretrain_routine_start", callback.on_pretrain_routine_start)
        model.add_callback("on_train_epoch_start", callback.on_train_epoch_start)
        model.add_callback("on_train_epoch_end", callback.on_train_epoch_end)
        model.add_callback("on_train_end", callback.on_train_end)

        # Quick dataset sanity check: verify a few images can be loaded
        try:
            import cv2
            data_path = Path(data_yaml).parent
            img_dirs = []
            for sub in ["images/train", "train/images", "images", "train"]:
                d = data_path / sub
                if d.exists():
                    img_dirs.append(d)
                    break
            checked = 0
            for img_dir in img_dirs:
                for img_path in list(img_dir.glob("*.jpg"))[:3] + list(img_dir.glob("*.png"))[:3]:
                    img = cv2.imread(str(img_path))
                    if img is None:
                        send_event({"type": "log", "data": {"message": f"[Warning] Failed to load image: {img_path}"}})
                    else:
                        checked += 1
            send_event({"type": "log", "data": {"message": f"[Debug] Dataset sanity check: {checked} images loaded successfully"}})
        except Exception as e:
            send_event({"type": "log", "data": {"message": f"[Debug] Dataset sanity check skipped: {e}"}})

        # Run training with watchdog
        send_event({"type": "log", "data": {"message": f"[Debug] train_args: {json.dumps({k: str(v) for k, v in train_args.items()})}"}})
        send_event({"type": "log", "data": {"message": "[Debug] About to call model.train()..."}})

        # Start a watchdog thread to detect if training is deadlocked (no progress for 10 min)
        watchdog_stop = threading.Event()
        def watchdog():
            deadlock_timeout = 600  # 10 minutes without any callback activity
            check_interval = 10     # check every 10 seconds
            while not watchdog_stop.is_set():
                time.sleep(check_interval)
                if watchdog_stop.is_set():
                    return
                idle = time.time() - callback.last_activity_time
                if idle > deadlock_timeout:
                    send_event({"type": "log", "data": {"message": f"[Watchdog] No training activity for {int(idle)}s. Callbacks fired: {callback.callback_fired}"}})
                    send_event({"type": "error", "error": f"Training deadlocked after {int(idle)}s of inactivity. Check dataset validity and device settings."})
                    return

        watchdog_thread = threading.Thread(target=watchdog, daemon=True)
        watchdog_thread.start()

        try:
            results = model.train(**train_args)
        finally:
            watchdog_stop.set()

        send_event({"type": "log", "data": {"message": "[Debug] model.train() returned"}})

        # Training complete
        best_path = None
        if hasattr(results, "best"):
            best_path = str(results.best)
        elif hasattr(model, "trainer") and hasattr(model.trainer, "best"):
            best_path = str(model.trainer.best)

        if best_path and os.path.exists(best_path):
            send_event({"type": "model_saved", "data": {"path": best_path}})

        send_event({"type": "complete", "data": {"final_metrics": {}}})

    except InterruptedError:
        send_event({"type": "stopped"})
    except Exception as e:
        error_msg = f"{type(e).__name__}: {str(e)}"
        tb = traceback.format_exc()
        send_event({"type": "error", "error": error_msg})
        send_event({"type": "log", "data": {"message": f"Error: {error_msg}"}})
        for line in tb.split("\n"):
            if line.strip():
                send_event({"type": "log", "data": {"message": line}})
    finally:
        _current_trainer = None


def main():
    """Main loop: read JSON commands from stdin."""
    # Check ultralytics availability
    has_ultralytics, version = check_ultralytics()
    if not has_ultralytics:
        send_event({"type": "error", "error": "ultralytics package not installed. Please run: pip install ultralytics"})
        sys.exit(1)

    send_event({"type": "log", "data": {"message": f"YOLO sidecar ready (ultralytics {version})"}})

    current_thread = None

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            cmd = json.loads(line)
        except json.JSONDecodeError:
            send_event({"type": "error", "error": f"Invalid JSON: {line[:100]}"})
            continue

        msg_type = cmd.get("type", "")

        if msg_type == "start":
            config = cmd.get("config", {})

            # Stop any existing training
            global _stop_requested
            _stop_requested = True
            if current_thread and current_thread.is_alive():
                current_thread.join(timeout=5)

            _stop_requested = False

            # Start training in a new thread
            current_thread = threading.Thread(target=run_training, args=(config,))
            current_thread.daemon = True
            current_thread.start()

        elif msg_type == "stop":
            _stop_requested = True
            send_event({"type": "log", "data": {"message": "Stop requested"}})

        else:
            send_event({"type": "error", "error": f"Unknown command type: {msg_type}"})


if __name__ == "__main__":
    main()
