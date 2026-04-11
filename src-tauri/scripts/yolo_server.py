#!/usr/bin/env python3
"""
YOLO Training Server - TCP socket + stdin pipe communication
Rust creates a TCP listener, passes the port as argv[1].
Python connects to TCP for sending events, reads commands from stdin.

This avoids stdout pollution from Ultralytics training output.

Following Ultralytics official callback pattern:
https://docs.ultralytics.com/modes/train/#callbacks
"""
import sys
import json
import os
import signal
import socket
import threading
import time
import traceback
from pathlib import Path

sys.stderr.reconfigure(line_buffering=True)

_tcp_socket = None
_tcp_lock = threading.Lock()

if len(sys.argv) > 1:
    try:
        _tcp_port = int(sys.argv[1])
        _tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        _tcp_socket.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        _tcp_socket.connect(('127.0.0.1', _tcp_port))
        print(f"Connected to Rust on port {_tcp_port}", file=sys.stderr, flush=True)
    except Exception as e:
        print(f"TCP connection failed: {e}", file=sys.stderr, flush=True)
        _tcp_socket = None

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


class TrainerState:
    def __init__(self):
        self.running = False
        self.model = None
        self.project_path = None
        self.stop_event = threading.Event()
        self.current_epoch = 0
        self.total_epochs = 0
        self.current_metrics = {}
        self.batch_count = 0
        self.total_batches = 0
        self.last_batch_event_time = 0
        self.heartbeat_stop = threading.Event()

_state = TrainerState()


def _heartbeat_thread():
    """Send heartbeat every 5 seconds so Rust knows Python is alive."""
    while not _state.heartbeat_stop.is_set():
        _state.heartbeat_stop.wait(5.0)
        if _state.heartbeat_stop.is_set():
            break
        try:
            send_event("heartbeat", {
                "running": _state.running,
                "epoch": _state.current_epoch,
                "total_epochs": _state.total_epochs,
                "batch": _state.batch_count,
                "total_batches": _state.total_batches,
            })
        except Exception as e:
            print(f"DEBUG: Heartbeat send failed: {e}", flush=True, file=sys.stderr)


def send_event(event_type, data=None, error=None):
    """Send JSON event via TCP socket (fallback to stdout if not connected)."""
    event = {"type": event_type}
    if data is not None:
        event["data"] = data
    if error is not None:
        event["error"] = error
    event_str = json.dumps(event) + '\n'
    if _tcp_socket:
        with _tcp_lock:
            try:
                _tcp_socket.sendall(event_str.encode('utf-8'))
            except Exception as e:
                print(f"TCP send failed: {e}", file=sys.stderr, flush=True)
    else:
        print(json.dumps(event), flush=True)


def log(msg):
    """Send log message."""
    send_event("log", {"message": msg})


def _to_float(value, default=0.0):
    try:
        if value is None:
            return default
        if hasattr(value, "item"):
            value = value.item()
        return float(value)
    except Exception:
        return default


def _extract_metric_value(metrics, *keys):
    if metrics is None:
        return 0.0

    candidates = [metrics]
    results_dict = getattr(metrics, "results_dict", None)
    if isinstance(results_dict, dict):
        candidates.append(results_dict)

    for candidate in candidates:
        if isinstance(candidate, dict):
            for key in keys:
                if key in candidate:
                    return _to_float(candidate.get(key))
        for key in keys:
            attr_name = key.replace("/", "_").replace("(", "").replace(")", "").replace("-", "_")
            if hasattr(candidate, attr_name):
                return _to_float(getattr(candidate, attr_name))
            if hasattr(candidate, key):
                return _to_float(getattr(candidate, key))

    return 0.0


def _extract_losses(trainer):
    loss_items = getattr(trainer, "loss_items", None)
    if callable(loss_items):
        loss_items = loss_items()
    if hasattr(loss_items, "tolist"):
        loss_items = loss_items.tolist()
    if isinstance(loss_items, (list, tuple)) and len(loss_items) >= 3:
        return (
            _to_float(loss_items[0]),
            _to_float(loss_items[1]),
            _to_float(loss_items[2]),
        )

    total_loss = getattr(trainer, "tloss", None)
    if hasattr(total_loss, "tolist"):
        total_loss = total_loss.tolist()
    if isinstance(total_loss, (list, tuple)) and len(total_loss) >= 3:
        return (
            _to_float(total_loss[0]),
            _to_float(total_loss[1]),
            _to_float(total_loss[2]),
        )

    return 0.0, 0.0, 0.0


def _extract_learning_rate(trainer):
    lr = getattr(trainer, "lr", None)
    if isinstance(lr, dict) and lr:
        values = [_to_float(value) for value in lr.values()]
        return values[-1] if values else 0.0
    return _to_float(lr)


def _build_epoch_data(trainer):
    box_loss, cls_loss, dfl_loss = _extract_losses(trainer)
    metrics = getattr(trainer, "metrics", None)

    return {
        "epoch": trainer.epoch + 1,
        "total_epochs": _state.total_epochs,
        "box_loss": box_loss,
        "cls_loss": cls_loss,
        "dfl_loss": dfl_loss,
        "val_box_loss": _extract_metric_value(metrics, "val/box_loss", "box_loss"),
        "val_cls_loss": _extract_metric_value(metrics, "val/cls_loss", "cls_loss"),
        "val_dfl_loss": _extract_metric_value(metrics, "val/dfl_loss", "dfl_loss"),
        "precision": _extract_metric_value(metrics, "metrics/precision(B)", "precision"),
        "recall": _extract_metric_value(metrics, "metrics/recall(B)", "recall"),
        "mAP50": _extract_metric_value(metrics, "metrics/mAP50(B)", "map50", "mAP50"),
        "mAP50-95": _extract_metric_value(metrics, "metrics/mAP50-95(B)", "map", "map50_95", "mAP50-95"),
        "learning_rate": _extract_learning_rate(trainer),
    }


def _safe_callback(name, func):
    """Wrap a callback with try/except to prevent callback errors from crashing training."""
    def wrapper(*args, **kwargs):
        try:
            func(*args, **kwargs)
        except Exception as e:
            tb = traceback.format_exc()
            print(f"DEBUG: Callback {name} error: {e}\n{tb}", flush=True, file=sys.stderr)
    return wrapper


# ========== YOLO Callbacks (Ultralytics official pattern) ==========

def _on_pretrain_routine_start(trainer):
    log("Initializing training...")


def _on_pretrain_routine_end(trainer):
    log("Initialization complete, starting training...")


def _on_train_start(trainer):
    """Called when training starts."""
    global _state
    _state.running = True
    _state.total_epochs = trainer.epochs
    _state.total_batches = getattr(trainer, 'num_train_batches', 0)
    if _state.total_batches == 0:
        try:
            train_loader = getattr(trainer, 'train_loader', None)
            if train_loader is not None:
                _state.total_batches = len(train_loader)
        except Exception:
            pass
    print(f"DEBUG: on_train_start called, epochs={trainer.epochs}, total_batches={_state.total_batches}", flush=True, file=sys.stderr)
    log(f"Training started: {_state.total_epochs} epochs")

    cuda_available = False
    cuda_version = None
    try:
        import torch
        cuda_available = torch.cuda.is_available()
        if cuda_available:
            cuda_version = torch.version.cuda
    except ImportError:
        pass

    send_event("started", {
        "total_epochs": _state.total_epochs,
        "device": str(trainer.device),
        "cuda_available": cuda_available,
        "cuda_version": cuda_version,
    })


def _on_train_epoch_start(trainer):
    """Called at the start of each epoch."""
    global _state
    _state.batch_count = 0
    if _state.total_batches == 0:
        try:
            train_loader = getattr(trainer, 'train_loader', None)
            if train_loader is not None:
                _state.total_batches = len(train_loader)
        except Exception:
            pass
    log(f"Epoch {trainer.epoch + 1}/{_state.total_epochs} starting...")


def _on_train_batch_end(trainer):
    """Called after each training batch - provides frequent progress updates."""
    global _state
    if not _state.running:
        return

    _state.batch_count += 1
    now = time.time()

    if now - _state.last_batch_event_time < 2.0 and _state.batch_count < _state.total_batches:
        return

    _state.last_batch_event_time = now

    box_loss, cls_loss, dfl_loss = _extract_losses(trainer)
    lr = _extract_learning_rate(trainer)

    batch_data = {
        "epoch": trainer.epoch + 1,
        "total_epochs": _state.total_epochs,
        "batch": _state.batch_count,
        "total_batches": _state.total_batches,
        "box_loss": box_loss,
        "cls_loss": cls_loss,
        "dfl_loss": dfl_loss,
        "learning_rate": lr,
    }

    send_event("batch_progress", batch_data)


def _on_train_epoch_end(trainer):
    """Called after each training epoch."""
    global _state
    print(f"DEBUG: on_train_epoch_end called, running={_state.running}, epoch={trainer.epoch}", flush=True, file=sys.stderr)

    if not _state.running:
        print(f"DEBUG: on_train_epoch_end early return - not running", flush=True, file=sys.stderr)
        return

    if _state.stop_event.is_set():
        log("Stop requested, halting training...")
        _state.running = False
        if _state.model and hasattr(_state.model, 'stop'):
            _state.model.stop()


def _on_fit_epoch_end(trainer):
    """Called after each full epoch (train + val)."""
    global _state
    if not _state.running:
        return

    try:
        epoch_data = _build_epoch_data(trainer)
        _state.current_epoch = epoch_data["epoch"]
        _state.current_metrics = epoch_data
        log(f"Epoch {_state.current_epoch}/{_state.total_epochs} completed")
        send_event("progress", epoch_data)
        
        # Check if this is the final epoch (all epochs completed)
        if _state.current_epoch >= _state.total_epochs:
            print("DEBUG: All epochs completed, calling _on_train_end", flush=True, file=sys.stderr)
            _on_train_end(trainer)
    except Exception as e:
        print(f"DEBUG: Failed to build epoch data: {e}", flush=True, file=sys.stderr)


def _on_val_start(trainer):
    pass


def _on_val_end(trainer):
    pass


def _on_train_end(trainer):
    """Called when training ends."""
    global _state
    print("DEBUG: _on_train_end called", flush=True, file=sys.stderr)
    _state.running = False
    _state.heartbeat_stop.set()
    log("Training completed")

    best_model = None
    # Try to get save_dir from trainer if available
    if trainer is not None:
        save_dir = getattr(trainer, 'save_dir', None)
        print(f"DEBUG: trainer.save_dir = {save_dir}", flush=True, file=sys.stderr)
    else:
        print("DEBUG: trainer is None, using fallback", flush=True, file=sys.stderr)
        # Fallback: try to find the most recent training output directory
        save_dir = None
        project_path = getattr(_state, 'project_path', None)
        print(f"DEBUG: project_path = {project_path}", flush=True, file=sys.stderr)
        if project_path:
            project_path = Path(project_path)
            # Look for the most recent train directory
            for exp_dir in sorted(project_path.glob("train*"), reverse=True):
                if exp_dir.is_dir():
                    save_dir = str(exp_dir)
                    print(f"DEBUG: Found train dir: {save_dir}", flush=True, file=sys.stderr)
                    break
    
    if save_dir:
        best_path = Path(save_dir) / "weights" / "best.pt"
        print(f"DEBUG: Checking best.pt at {best_path}, exists={best_path.exists()}", flush=True, file=sys.stderr)
        if best_path.exists():
            best_model = str(best_path)
            print(f"DEBUG: Found best model at {best_model}", flush=True, file=sys.stderr)
        else:
            print(f"DEBUG: best.pt does not exist", flush=True, file=sys.stderr)
    else:
        print("DEBUG: save_dir is None", flush=True, file=sys.stderr)

    final_metrics = _state.current_metrics.copy()
    final_metrics["model_path"] = best_model

    print(f"DEBUG: Sending complete event with model_path={best_model}", flush=True, file=sys.stderr)
    send_event("complete", {
        "success": True,
        "model_path": best_model,
        "final_metrics": final_metrics,
    })
    print("DEBUG: complete event sent", flush=True, file=sys.stderr)


def _on_model_save(trainer):
    """Called when model is saved."""
    save_dir = getattr(trainer, 'save_dir', None)
    if save_dir:
        best_path = Path(save_dir) / "weights" / "best.pt"
        if best_path.exists():
            log(f"Model saved: {best_path}")
            send_event("model_saved", {"path": str(best_path)})


def _on_error(trainer):
    """Called when an error occurs."""
    global _state
    _state.running = False
    _state.heartbeat_stop.set()
    error_msg = "Unknown error occurred during training"
    send_event("error", error=error_msg)


def _on_exception(trainer, exception):
    """Called when an exception is raised."""
    global _state
    _state.running = False
    _state.heartbeat_stop.set()
    send_event("error", error=str(exception))


CALLBACKS = {
    'on_pretrain_routine_start': _safe_callback('on_pretrain_routine_start', _on_pretrain_routine_start),
    'on_pretrain_routine_end': _safe_callback('on_pretrain_routine_end', _on_pretrain_routine_end),
    'on_train_start': _safe_callback('on_train_start', _on_train_start),
    'on_train_epoch_start': _safe_callback('on_train_epoch_start', _on_train_epoch_start),
    'on_train_batch_end': _safe_callback('on_train_batch_end', _on_train_batch_end),
    'on_train_epoch_end': _safe_callback('on_train_epoch_end', _on_train_epoch_end),
    'on_fit_epoch_end': _safe_callback('on_fit_epoch_end', _on_fit_epoch_end),
    'on_val_start': _safe_callback('on_val_start', _on_val_start),
    'on_val_end': _safe_callback('on_val_end', _on_val_end),
    'on_train_end': _safe_callback('on_train_end', _on_train_end),
    'on_model_save': _safe_callback('on_model_save', _on_model_save),
    'on_error': _safe_callback('on_error', _on_error),
    'on_exception': _safe_callback('on_exception', _on_exception),
}


def main():
    global _state
    _state = TrainerState()

    send_event("connected", {"pid": os.getpid()})

    hb_thread = threading.Thread(target=_heartbeat_thread, daemon=True)
    hb_thread.start()

    def handle_command(cmd):
        """Handle a command from stdin."""
        global _state
        cmd_type = cmd.get("type")

        if cmd_type == "start":
            config = cmd.get("config", {})
            log(f"Received start command: epochs={config.get('epochs')}")

            project_path = config.get("project_path")
            if not project_path:
                send_event("error", error="project_path is required")
                return
            
            # Save project_path for fallback in _on_train_end
            _state.project_path = project_path

            data_yaml = Path(project_path) / "data.yaml"
            if not data_yaml.exists():
                send_event("error", error=f"data.yaml not found at {data_yaml}")
                return

            base_model = config.get("base_model", "yolo11n.pt")

            try:
                _state.model = YOLO(base_model)
                log(f"Model loaded: {base_model}")
            except Exception as e:
                send_event("error", error=f"Failed to load model: {e}")
                return

            device = config.get("device", 0)
            try:
                import torch
                if isinstance(device, int) and device < 0:
                    device = "cpu"
                elif isinstance(device, int) and device >= 0:
                    device = str(device)
                    if not torch.cuda.is_available():
                        log("CUDA not available, using CPU")
                        device = "cpu"
                elif device == "cpu" or not torch.cuda.is_available():
                    log("CUDA not available, using CPU")
                    device = "cpu"
            except ImportError:
                device = "cpu"

            train_args = {
                "data": str(data_yaml),
                "epochs": config.get("epochs", 50),
                "batch": config.get("batch_size", 16),
                "imgsz": config.get("image_size", 640),
                "device": device,
                "workers": 0,
                "optimizer": config.get("optimizer", "SGD"),
                "project": project_path,
                "name": "train",
                "exist_ok": True,
                "lr0": config.get("lr0", 0.01),
                "lrf": config.get("lrf", 0.01),
                "momentum": config.get("momentum", 0.937),
                "weight_decay": config.get("weight_decay", 0.0005),
                "warmup_epochs": config.get("warmup_epochs", 3.0),
                "warmup_bias_lr": config.get("warmup_bias_lr", 0.1),
                "warmup_momentum": config.get("warmup_momentum", 0.8),
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
                "close_mosaic": config.get("close_mosaic", 10),
                "rect": config.get("rect", False),
                "cos_lr": config.get("cos_lr", False),
                "single_cls": config.get("single_cls", False),
                "amp": config.get("amp", True),
                "save_period": config.get("save_period", -1),
                "cache": config.get("cache", False),
            }

            def training_worker():
                global _state
                try:
                    _state.running = True
                    _state.stop_event.clear()
                    _state.heartbeat_stop.clear()
                    print("DEBUG: Training worker starting", flush=True, file=sys.stderr)

                    for name, callback in CALLBACKS.items():
                        _state.model.add_callback(name, callback)
                    print("DEBUG: Callbacks registered on model", flush=True, file=sys.stderr)

                    model_path = Path(base_model)
                    print(f"DEBUG: model_path={model_path}", flush=True, file=sys.stderr)
                    print(f"DEBUG: model exists={model_path.exists()}", flush=True, file=sys.stderr)
                    print(f"DEBUG: data_yaml={data_yaml}", flush=True, file=sys.stderr)
                    print(f"DEBUG: data_yaml exists={data_yaml.exists()}", flush=True, file=sys.stderr)
                    print(f"DEBUG: train_args={train_args}", flush=True, file=sys.stderr)

                    log("About to call model.train()...")
                    print("DEBUG: Calling model.train()", flush=True, file=sys.stderr)

                    results = _state.model.train(**train_args)

                    print("DEBUG: model.train() returned", flush=True, file=sys.stderr)
                    log(f"Training finished: {results}")
                    
                    # Ensure complete event is sent even if YOLO callback didn't fire
                    print("DEBUG: Calling _on_train_end to ensure complete event", flush=True, file=sys.stderr)
                    trainer = getattr(results, 'trainer', None) if results else None
                    _on_train_end(trainer)

                except Exception as e:
                    _state.running = False
                    _state.heartbeat_stop.set()
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
            _state.heartbeat_stop.set()
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
            _state.heartbeat_stop.set()
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

    log("YOLO training server started")

    def signal_handler(sig, frame):
        global _state
        log(f"Received signal {sig}")
        _state.stop_event.set()
        _state.heartbeat_stop.set()
        _state.running = False
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

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
