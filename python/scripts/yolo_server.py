#!/usr/bin/env python3
"""
YOLO Training Server - stdin/stdout pipe communication
Rust sends JSON commands via stdin, Python sends JSON events via stdout.
Python actively reports progress, errors, and status via callbacks.
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


class YoloTrainer:
    """YOLO trainer with active status reporting via callbacks."""

    def __init__(self):
        self.running = False
        self.model = None
        self.stop_event = threading.Event()
        self.current_epoch = 0
        self.total_epochs = 0
        self.current_metrics = {}

    def send_event(self, event_type, data=None, error=None):
        """Send JSON event to stdout (non-blocking)."""
        event = {"type": event_type}
        if data is not None:
            event["data"] = data
        if error is not None:
            event["error"] = error
        # Use flush=True to ensure immediate delivery
        print(json.dumps(event), flush=True)

    def log(self, msg):
        """Send log message."""
        self.send_event("log", {"message": msg})

    # ========== YOLO Callbacks ==========

    def on_pretrain_routine_start(self, trainer):
        self.log("Initializing training...")

    def on_pretrain_routine_end(self, trainer):
        self.log("Initialization complete, starting training...")

    def on_train_start(self, trainer):
        self.running = True
        self.total_epochs = trainer.epochs
        print(f"DEBUG: on_train_start called, trainer.epochs={trainer.epochs}", flush=True, file=sys.stderr)
        self.log(f"Training started: {self.total_epochs} epochs")
        self.send_event("started", {
            "total_epochs": self.total_epochs,
            "device": str(trainer.device),
        })

    def on_train_epoch_start(self, trainer):
        # trainer.epoch is 0-indexed, so add 1 for 1-indexed reporting (consistent with on_train_epoch_end)
        epoch = trainer.epoch + 1
        print(f"DEBUG: on_train_epoch_start called, epoch={epoch} (trainer.epoch was {trainer.epoch})", flush=True, file=sys.stderr)

    def on_train_epoch_end(self, trainer):
        """Called after each epoch - this is our main progress source."""
        print(f"DEBUG: on_train_epoch_end called, running={self.running}, trainer.epoch={trainer.epoch}", flush=True, file=sys.stderr)
        if not self.running:
            print(f"DEBUG: on_train_epoch_end early return - not running", flush=True, file=sys.stderr)
            return

        # trainer.epoch is 0-indexed, so add 1 for 1-indexed reporting
        self.current_epoch = trainer.epoch + 1
        print(f"DEBUG: on_train_epoch_end sending progress for epoch {self.current_epoch}/{self.total_epochs} (trainer.epoch was {trainer.epoch})", flush=True, file=sys.stderr)
        self.log(f"Epoch {self.current_epoch}/{self.total_epochs} completed, sending progress...")

        # Extract metrics from trainer
        metrics = getattr(trainer, 'metrics', None)

        epoch_data = {
            "epoch": self.current_epoch,
            "total_epochs": self.total_epochs,
        }

        # Extract loss metrics safely - skip if uncertain to avoid serialization errors
        try:
            if hasattr(trainer, 'loss_items'):
                loss_items = trainer.loss_items
                # If it's callable (method), call it; otherwise use directly
                if callable(loss_items):
                    loss_items = loss_items()
                # Now try to convert and extract
                if loss_items is not None:
                    try:
                        # Try to convert tensor to list
                        if hasattr(loss_items, 'tolist'):
                            loss_items = loss_items.tolist()
                        # Check if it's a sequence we can index
                        if isinstance(loss_items, (list, tuple)) and len(loss_items) >= 3:
                            epoch_data["box_loss"] = float(loss_items[0])
                            epoch_data["cls_loss"] = float(loss_items[1])
                            epoch_data["dfl_loss"] = float(loss_items[2])
                            self.log(f"Loss values: box={epoch_data['box_loss']}, cls={epoch_data['cls_loss']}, dfl={epoch_data['dfl_loss']}")
                    except (TypeError, IndexError, ValueError) as e:
                        self.log(f"Failed to extract loss items: {e}")
                        pass  # Skip loss items if conversion fails
        except Exception as e:
            self.log(f"Failed to access loss_items: {e}")
            pass  # Skip entirely if loss_items access fails

        # Extract validation metrics safely
        if metrics is not None:
            try:
                epoch_data["precision"] = float(getattr(metrics, 'precision', 0) or 0)
                epoch_data["recall"] = float(getattr(metrics, 'recall', 0) or 0)
                epoch_data["mAP50"] = float(getattr(metrics, 'map50', 0) or 0)
                epoch_data["mAP50-95"] = float(getattr(metrics, 'map50-95', 0) or 0)
            except Exception:
                pass

        self.current_metrics = epoch_data
        self.log(f"Sending progress event: {epoch_data}")
        self.send_event("progress", epoch_data)

        # Check if stop requested
        if self.stop_event.is_set():
            self.log("Stop requested, halting training...")
            self.running = False
            if hasattr(self.model, 'stop'):
                self.model.stop()

    def on_val_start(self, trainer):
        pass

    def on_val_end(self, trainer):
        pass

    def on_train_end(self, trainer):
        """Training completed."""
        self.running = False
        self.log("Training completed")

        # Find the best model
        best_model = None
        save_dir = getattr(trainer, 'save_dir', None)
        if save_dir:
            best_path = Path(save_dir) / "weights" / "best.pt"
            if best_path.exists():
                best_model = str(best_path)

        final_metrics = self.current_metrics.copy()
        final_metrics["model_path"] = best_model

        self.send_event("complete", {
            "success": True,
            "model_path": best_model,
            "final_metrics": final_metrics,
        })

    def on_model_save(self, trainer):
        """Called when model is saved (e.g., best.pt)."""
        save_dir = getattr(trainer, 'save_dir', None)
        if save_dir:
            best_path = Path(save_dir) / "weights" / "best.pt"
            if best_path.exists():
                self.log(f"Model saved: {best_path}")
                self.send_event("model_saved", {"path": str(best_path)})

    def on_error(self, trainer):
        """Called when an error occurs during training."""
        self.running = False
        error_msg = "Unknown error occurred during training"
        self.send_event("error", error=error_msg)

    def on_exception(self, trainer, exception):
        """Called when an exception is raised."""
        self.running = False
        self.send_event("error", error=str(exception))


def main():
    trainer = YoloTrainer()

    # Register callbacks
    callbacks = {
        'on_pretrain_routine_start': trainer.on_pretrain_routine_start,
        'on_pretrain_routine_end': trainer.on_pretrain_routine_end,
        'on_train_start': trainer.on_train_start,
        'on_train_epoch_start': trainer.on_train_epoch_start,
        'on_train_epoch_end': trainer.on_train_epoch_end,
        'on_val_start': trainer.on_val_start,
        'on_val_end': trainer.on_val_end,
        'on_train_end': trainer.on_train_end,
        'on_model_save': trainer.on_model_save,
        'on_error': trainer.on_error,
        'on_exception': trainer.on_exception,
    }

    def handle_command(cmd):
        """Handle a command from stdin."""
        cmd_type = cmd.get("type")

        if cmd_type == "start":
            config = cmd.get("config", {})
            trainer.log(f"Received start command: epochs={config.get('epochs')}")

            # Validate project path
            project_path = config.get("project_path")
            if not project_path:
                trainer.send_event("error", error="project_path is required")
                return

            data_yaml = Path(project_path) / "data.yaml"
            if not data_yaml.exists():
                trainer.send_event("error", error=f"data.yaml not found at {data_yaml}")
                return

            base_model = config.get("base_model", "yolo11n.pt")

            # Load model
            try:
                trainer.model = YOLO(base_model)
                trainer.log(f"Model loaded: {base_model}")
            except Exception as e:
                trainer.send_event("error", error=f"Failed to load model: {e}")
                return

            # Auto-detect device
            device = config.get("device", 0)
            try:
                import torch
                if not torch.cuda.is_available():
                    trainer.log("CUDA not available, using CPU")
                    device = "cpu"
            except ImportError:
                device = "cpu"

            # Prepare training arguments (NO callbacks here - register them on model instead)
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
            }

            # Send started event
            trainer.send_event("started", {"config": train_args})

            # Run training in background thread
            def training_worker():
                import traceback
                import sys
                try:
                    trainer.running = True
                    trainer.stop_event.clear()
                    print("DEBUG: Training worker starting", flush=True, file=sys.stderr)

                    # Register callbacks on the model BEFORE training
                    for name, callback in callbacks.items():
                        trainer.model.add_callback(name, callback)
                    print("DEBUG: Callbacks registered on model", flush=True, file=sys.stderr)
                    # Also register callbacks on the trainer object if it exists
                    if hasattr(trainer.model, 'trainer') and trainer.model.trainer is not None:
                        for name, callback in callbacks.items():
                            trainer.model.trainer.add_callback(name, callback)
                        print("DEBUG: Callbacks also registered on model.trainer", flush=True, file=sys.stderr)

                    # Check model exists - use base_model from closure
                    model_path = Path(base_model)
                    print(f"DEBUG: model_path={model_path}", flush=True, file=sys.stderr)
                    print(f"DEBUG: model exists={model_path.exists()}", flush=True, file=sys.stderr)

                    # Check data file exists
                    print(f"DEBUG: data_yaml={data_yaml}", flush=True, file=sys.stderr)
                    print(f"DEBUG: data_yaml exists={data_yaml.exists()}", flush=True, file=sys.stderr)

                    # Print train_args keys
                    print(f"DEBUG: train_args keys={list(train_args.keys())}", flush=True, file=sys.stderr)
                    print(f"DEBUG: train_args epochs={train_args.get('epochs')}", flush=True, file=sys.stderr)
                    print(f"DEBUG: train_args data={train_args.get('data')}", flush=True, file=sys.stderr)

                    trainer.log("About to call model.train()...")
                    print("DEBUG: Calling model.train()", flush=True, file=sys.stderr)
                    results = trainer.model.train(**train_args)
                    print("DEBUG: model.train() returned", flush=True, file=sys.stderr)
                    trainer.log(f"Training finished: {results}")
                except Exception as e:
                    trainer.running = False
                    tb = traceback.format_exc()
                    print(f"DEBUG: Exception in training_worker: {e}", flush=True, file=sys.stderr)
                    print(f"DEBUG: Traceback: {tb}", flush=True, file=sys.stderr)
                    trainer.log(f"Training error: {e}")
                    trainer.log(f"Traceback: {tb}")
                    trainer.send_event("error", error=f"{e}\n{tb}")

            thread = threading.Thread(target=training_worker, daemon=True)
            thread.start()

        elif cmd_type == "stop":
            print("DEBUG: Received stop command", flush=True, file=sys.stderr)
            trainer.log("Received stop command")
            trainer.stop_event.set()
            if trainer.running and trainer.model:
                try:
                    trainer.model.stop()
                except Exception:
                    pass
            trainer.running = False
            trainer.send_event("stopped")
            print("DEBUG: Stop command processed, exiting", flush=True, file=sys.stderr)
            sys.exit(0)

        elif cmd_type == "quit":
            trainer.log("Received quit command")
            trainer.stop_event.set()
            trainer.running = False
            trainer.send_event("quit")
            sys.exit(0)

        elif cmd_type == "video_inference":
            config = cmd.get("config", {})
            video_path = config.get("video_path")
            model_name = config.get("model", "yolo11n.pt")
            conf = config.get("conf", 0.25)
            iou = config.get("iou", 0.45)
            save = config.get("save", True)
            output_dir = config.get("output_dir")
            
            if not video_path or not Path(video_path).exists():
                trainer.send_event("error", error=f"Video not found: {video_path}")
                return
            
            def video_worker():
                try:
                    trainer.running = True
                    trainer.log(f"Loading model: {model_name}")
                    model = YOLO(model_name)
                    
                    kwargs = {
                        "conf": conf,
                        "iou": iou,
                        "save": save,
                        "stream": True,
                        "verbose": False,
                    }
                    if output_dir:
                        kwargs["project"] = output_dir
                    
                    trainer.log(f"Starting video inference: {video_path}")
                    trainer.send_event("started", {"total_frames": 0})
                    
                    frame_count = 0
                    total_detections = 0
                    for result in model(video_path, **kwargs):
                        frame_count += 1
                        dets = len(result.boxes) if result.boxes is not None else 0
                        total_detections += dets
                        
                        if frame_count % 10 == 0:
                            trainer.send_event("progress", {
                                "frame": frame_count,
                                "detections": total_detections,
                            })
                        
                        if trainer.stop_event.is_set():
                            trainer.log("Video inference stopped")
                            break
                    
                    trainer.running = False
                    trainer.send_event("complete", {
                        "success": True,
                        "total_frames": frame_count,
                        "total_detections": total_detections,
                    })
                except Exception as e:
                    trainer.running = False
                    import traceback
                    trainer.send_event("error", error=f"{e}\n{traceback.format_exc()}")
            
            thread = threading.Thread(target=video_worker, daemon=True)
            thread.start()

        elif cmd_type == "status":
            status = {
                "running": trainer.running,
                "epoch": trainer.current_epoch,
                "total_epochs": trainer.total_epochs,
                "metrics": trainer.current_metrics,
            }
            trainer.send_event("status", status)

        else:
            trainer.send_event("error", error=f"Unknown command type: {cmd_type}")

    # Main loop - read commands from stdin
    trainer.log("YOLO training server started")

    # Set signal handlers for graceful shutdown
    def signal_handler(sig, frame):
        trainer.log(f"Received signal {sig}")
        trainer.stop_event.set()
        trainer.running = False
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
            trainer.send_event("error", error=f"Invalid JSON: {e}")
        except Exception as e:
            trainer.send_event("error", error=str(e))


if __name__ == "__main__":
    main()
