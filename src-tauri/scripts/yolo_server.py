#!/usr/bin/env python3
"""
YOLO Training Server - stdin/stdout pipe communication
Rust sends JSON commands via stdin, Python sends JSON events via stdout.
Python actively reports progress, errors, and status via callbacks.

Following Ultralytics official callback pattern:
https://docs.ultralytics.com/modes/train/#callbacks
"""
import sys
import json
import os
import signal
import threading
from pathlib import Path

# Force unbuffered mode
sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

print("YOLO server starting...", file=sys.stderr, flush=True)

try:
    from ultralytics import YOLO
    print("Ultralytics imported successfully", file=sys.stderr, flush=True)
except ImportError as e:
    print(f"ERROR: Failed to import ultralytics: {e}", file=sys.stderr, flush=True)
    sys.exit(1)
except Exception as e:
    print(f"ERROR: Unexpected error during import: {e}", file=sys.stderr, flush=True)
    sys.exit(1)

print("YOLO server initialized", file=sys.stderr, flush=True)


# Global trainer state - shared by all callbacks (following Ultralytics pattern)
class TrainerState:
    def __init__(self):
        self.running = False
        self.model = None
        self.stop_event = threading.Event()
        self.current_epoch = 0
        self.total_epochs = 0
        self.current_metrics = {}

_state = TrainerState()


def send_event(event_type, data=None, error=None):
    """Send JSON event to stdout (non-blocking)."""
    event = {"type": event_type}
    if data is not None:
        event["data"] = data
    if error is not None:
        event["error"] = error
    print(json.dumps(event), flush=True)


def log(msg):
    """Send log message."""
    send_event("log", {"message": msg})


# ========== YOLO Callbacks (Ultralytics official pattern) ==========
# These are global functions, not instance methods

def on_pretrain_routine_start(trainer):
    log("Initializing training...")


def on_pretrain_routine_end(trainer):
    log("Initialization complete, starting training...")


def on_train_start(trainer):
    """Called when training starts."""
    global _state
    _state.running = True
    _state.total_epochs = trainer.epochs
    print(f"DEBUG: on_train_start called, epochs={trainer.epochs}", flush=True, file=sys.stderr)
    log(f"Training started: {_state.total_epochs} epochs")
    send_event("started", {
        "total_epochs": _state.total_epochs,
        "device": str(trainer.device),
    })


def on_train_epoch_start(trainer):
    """Called at the start of each epoch."""
    pass


def on_train_epoch_end(trainer):
    """Called after each epoch - main progress callback."""
    global _state
    print(f"DEBUG: on_train_epoch_end called, running={_state.running}, epoch={trainer.epoch}", flush=True, file=sys.stderr)

    if not _state.running:
        print(f"DEBUG: on_train_epoch_end early return - not running", flush=True, file=sys.stderr)
        return

    # trainer.epoch is 0-indexed
    _state.current_epoch = trainer.epoch + 1

    epoch_data = {
        "epoch": _state.current_epoch,
        "total_epochs": _state.total_epochs,
    }

    # Extract loss metrics safely
    try:
        loss_items = getattr(trainer, 'loss_items', None)
        if loss_items is not None:
            if callable(loss_items):
                loss_items = loss_items()
            if hasattr(loss_items, 'tolist'):
                loss_items = loss_items.tolist()
            if isinstance(loss_items, (list, tuple)) and len(loss_items) >= 3:
                epoch_data["box_loss"] = float(loss_items[0])
                epoch_data["cls_loss"] = float(loss_items[1])
                epoch_data["dfl_loss"] = float(loss_items[2])
    except Exception as e:
        print(f"DEBUG: Failed to extract loss: {e}", flush=True, file=sys.stderr)

    # Extract validation metrics
    try:
        metrics = getattr(trainer, 'metrics', None)
        if metrics is not None:
            epoch_data["precision"] = float(getattr(metrics, 'precision', 0) or 0)
            epoch_data["recall"] = float(getattr(metrics, 'recall', 0) or 0)
            epoch_data["mAP50"] = float(getattr(metrics, 'map50', 0) or 0)
            epoch_data["mAP50-95"] = float(getattr(metrics, 'map50-95', 0) or 0)
    except Exception as e:
        print(f"DEBUG: Failed to extract metrics: {e}", flush=True, file=sys.stderr)

    _state.current_metrics = epoch_data
    log(f"Epoch {_state.current_epoch}/{_state.total_epochs} completed")
    send_event("progress", epoch_data)

    # Check if stop requested
    if _state.stop_event.is_set():
        log("Stop requested, halting training...")
        _state.running = False
        if _state.model and hasattr(_state.model, 'stop'):
            _state.model.stop()


def on_val_start(trainer):
    pass


def on_val_end(trainer):
    pass


def on_train_end(trainer):
    """Called when training ends."""
    global _state
    _state.running = False
    log("Training completed")

    # Find the best model
    best_model = None
    save_dir = getattr(trainer, 'save_dir', None)
    if save_dir:
        best_path = Path(save_dir) / "weights" / "best.pt"
        if best_path.exists():
            best_model = str(best_path)

    final_metrics = _state.current_metrics.copy()
    final_metrics["model_path"] = best_model

    send_event("complete", {
        "success": True,
        "model_path": best_model,
        "final_metrics": final_metrics,
    })


def on_model_save(trainer):
    """Called when model is saved."""
    save_dir = getattr(trainer, 'save_dir', None)
    if save_dir:
        best_path = Path(save_dir) / "weights" / "best.pt"
        if best_path.exists():
            log(f"Model saved: {best_path}")
            send_event("model_saved", {"path": str(best_path)})


def on_error(trainer):
    """Called when an error occurs."""
    global _state
    _state.running = False
    error_msg = "Unknown error occurred during training"
    send_event("error", error=error_msg)


def on_exception(trainer, exception):
    """Called when an exception is raised."""
    global _state
    _state.running = False
    send_event("error", error=str(exception))


# Ultralytics official callback map
CALLBACKS = {
    'on_pretrain_routine_start': on_pretrain_routine_start,
    'on_pretrain_routine_end': on_pretrain_routine_end,
    'on_train_start': on_train_start,
    'on_train_epoch_start': on_train_epoch_start,
    'on_train_epoch_end': on_train_epoch_end,
    'on_val_start': on_val_start,
    'on_val_end': on_val_end,
    'on_train_end': on_train_end,
    'on_model_save': on_model_save,
    'on_error': on_error,
    'on_exception': on_exception,
}


def main():
    # Reset state
    global _state
    _state = TrainerState()

    def handle_command(cmd):
        """Handle a command from stdin."""
        global _state
        cmd_type = cmd.get("type")

        if cmd_type == "start":
            config = cmd.get("config", {})
            log(f"Received start command: epochs={config.get('epochs')}")

            # Validate project path
            project_path = config.get("project_path")
            if not project_path:
                send_event("error", error="project_path is required")
                return

            data_yaml = Path(project_path) / "data.yaml"
            if not data_yaml.exists():
                send_event("error", error=f"data.yaml not found at {data_yaml}")
                return

            base_model = config.get("base_model", "yolo11n.pt")

            # Load model
            try:
                _state.model = YOLO(base_model)
                log(f"Model loaded: {base_model}")
            except Exception as e:
                send_event("error", error=f"Failed to load model: {e}")
                return

            # Auto-detect device
            device = config.get("device", 0)
            try:
                import torch
                # device_id: -1 = CPU, 0+ = GPU device number
                if isinstance(device, int) and device < 0:
                    device = "cpu"
                elif isinstance(device, int) and device >= 0:
                    device = str(device)  # Convert to string "0", "1", etc.
                    if not torch.cuda.is_available():
                        log("CUDA not available, using CPU")
                        device = "cpu"
                elif device == "cpu" or not torch.cuda.is_available():
                    log("CUDA not available, using CPU")
                    device = "cpu"
            except ImportError:
                device = "cpu"

            # Prepare training arguments — include ALL hyperparameters
            train_args = {
                "data": str(data_yaml),
                "epochs": config.get("epochs", 50),
                "batch": config.get("batch_size", 16),
                "imgsz": config.get("image_size", 640),
                "device": device,
                "workers": config.get("workers", 8),
                "optimizer": config.get("optimizer", "SGD"),
                "project": project_path,
                "name": "train",
                "exist_ok": True,
                # Hyperparameters
                "lr0": config.get("lr0", 0.01),
                "lrf": config.get("lrf", 0.01),
                "momentum": config.get("momentum", 0.937),
                "weight_decay": config.get("weight_decay", 0.0005),
                # Warmup
                "warmup_epochs": config.get("warmup_epochs", 3.0),
                "warmup_bias_lr": config.get("warmup_bias_lr", 0.1),
                "warmup_momentum": config.get("warmup_momentum", 0.8),
                # Augmentation
                "hsv_h": config.get("hsv_h", 0.015),
                "hsv_s": config.get("hsv_s", 0.7),
                "hsv_v": config.get("hsv_v", 0.4),
                "translate": config.get("translate", 0.1),
                "scale": config.get("scale", 0.5),
                "shear": config.get("shear", 0.0),
                "perspective": config.get("perspective", 0.0),
                "flipud": config.get("flipud", 0.0),
                "fliplr": config.get("fliplr", 0.5),
                "mosaic": config.get("mosaic", 1.0),
                "mixup": config.get("mixup", 0.0),
                "copy_paste": config.get("copy_paste", 0.0),
                # Training options
                "rect": config.get("rect", False),
                "cos_lr": config.get("cos_lr", False),
                "single_cls": config.get("single_cls", False),
                "amp": config.get("amp", True),
                "save_period": config.get("save_period", -1),
                "cache": config.get("cache", False),
            }

            # Send started event
            send_event("started", {"config": train_args})

            # Run training in background thread
            def training_worker():
                global _state
                import traceback
                try:
                    _state.running = True
                    _state.stop_event.clear()
                    print("DEBUG: Training worker starting", flush=True, file=sys.stderr)

                    # Register callbacks on the model (Ultralytics official pattern)
                    for name, callback in CALLBACKS.items():
                        _state.model.add_callback(name, callback)
                    print("DEBUG: Callbacks registered on model", flush=True, file=sys.stderr)

                    # Print debug info
                    model_path = Path(base_model)
                    print(f"DEBUG: model_path={model_path}", flush=True, file=sys.stderr)
                    print(f"DEBUG: model exists={model_path.exists()}", flush=True, file=sys.stderr)
                    print(f"DEBUG: data_yaml={data_yaml}", flush=True, file=sys.stderr)
                    print(f"DEBUG: data_yaml exists={data_yaml.exists()}", flush=True, file=sys.stderr)
                    print(f"DEBUG: train_args={train_args}", flush=True, file=sys.stderr)

                    log("About to call model.train()...")
                    print("DEBUG: Calling model.train()", flush=True, file=sys.stderr)

                    # Train the model
                    results = _state.model.train(**train_args)
                    print("DEBUG: model.train() returned", flush=True, file=sys.stderr)
                    log(f"Training finished: {results}")

                except Exception as e:
                    _state.running = False
                    tb = traceback.format_exc()
                    print(f"DEBUG: Exception in training_worker: {e}", flush=True, file=sys.stderr)
                    print(f"DEBUG: Traceback: {tb}", flush=True, file=sys.stderr)
                    log(f"Training error: {e}")
                    send_event("error", error=f"{e}\n{tb}")

            thread = threading.Thread(target=training_worker, daemon=True)
            thread.start()

        elif cmd_type == "stop":
            print("DEBUG: Received stop command", flush=True, file=sys.stderr)
            log("Received stop command")
            _state.stop_event.set()
            if _state.running and _state.model:
                try:
                    _state.model.stop()
                except Exception:
                    pass
            _state.running = False
            send_event("stopped")
            print("DEBUG: Stop command processed, exiting", flush=True, file=sys.stderr)
            sys.exit(0)

        elif cmd_type == "quit":
            log("Received quit command")
            _state.stop_event.set()
            _state.running = False
            send_event("quit")
            sys.exit(0)

        elif cmd_type == "status":
            status = {
                "running": _state.running,
                "epoch": _state.current_epoch,
                "total_epochs": _state.total_epochs,
                "metrics": _state.current_metrics,
            }
            send_event("status", status)

        else:
            send_event("error", error=f"Unknown command type: {cmd_type}")

    # Main loop - read commands from stdin
    log("YOLO training server started")

    # Set signal handlers for graceful shutdown
    def signal_handler(sig, frame):
        global _state
        log(f"Received signal {sig}")
        _state.stop_event.set()
        _state.running = False
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Read commands
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            cmd = json.loads(line)
            handle_command(cmd)
        except json.JSONDecodeError as e:
            send_event("error", error=f"Invalid JSON: {e}")
        except Exception as e:
            send_event("error", error=str(e))


if __name__ == "__main__":
    main()
