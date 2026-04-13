# RustTools — YOLO Desktop App

## Project Stack
- **Backend**: Rust + Tauri 2.x, Burn framework for ML training
- **Frontend**: React 18 + TypeScript + Vite
- **ML**: YOLO models (YOLOv8/11/12), pure Rust inference via Burn/onnxruntime-rs
- **Styling**: CSS Modules (no Tailwind, no inline styles in new code)
- **State**: Zustand for frontend state management

## Key Directories
```
src/                          # React frontend
src/modules/yolo/pages/        # Page components (TrainingPage, VideoPage, etc.)
src/shared/components/ui/      # Shared UI (Toast, Modal, etc.)
src/core/stores/              # Zustand stores
src-tauri/src/                # Rust backend
src-tauri/src/modules/yolo/   # YOLO domain (commands, services)
src-tauri/src/modules/yolo/services/trainer.rs  # Burn training
src-tauri/src/modules/yolo/services/inference_core.rs  # Rust inference
```

## Critical Conventions
- **No Python env checks** — pure Rust with CUDA detection only
- **Proxy config**: `~/.config/rust-tools/proxy.json` (no hardcoded proxy URLs)
- **Model download**: GitHub release URLs + configurable proxy
- **num_classes**: always read from project's `data.yaml` (nc field), never hardcode
- **CSS Modules required** for all new page components (no inline `style={{}}`)
- **Toast over alert()** — use shared Toast component, never browser alert

## Build Commands
```bash
npm run dev          # Frontend dev
cargo build --manifest-path src-tauri/Cargo.toml  # Rust build
cargo check --manifest-path src-tauri/Cargo.toml  # Rust type check
npm run build       # Frontend production build
```

## Development Workflow
When asked to implement a feature or fix:
1. Read relevant files first
2. Check existing patterns in similar code
3. Follow the adversarial-dev-workflow skill for significant changes
4. Always verify with cargo check / npm run build before committing

## Style Rules
- TypeScript: strict mode, explicit types on all function signatures
- Rust: clippy-compliant, no unsafe unless necessary
- CSS: CSS Modules only, modern dark theme colors (bg: #0f0f1a, accent: #00d4ff)
- Components: functional React with hooks, no class components
