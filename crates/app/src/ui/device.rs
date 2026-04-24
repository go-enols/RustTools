use eframe::egui;
use crate::app::RustToolsApp;
use crate::theme::{AppleColors, card_frame, page_header};

#[derive(Default, Clone, Debug, serde::Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub cuda_version: String,
    pub cudnn_version: String,
}

#[derive(Default, Clone, Debug)]
pub struct DeviceInfoState {
    pub gpu_info: Option<Vec<GpuInfo>>,
    pub cpu_model: String,
    pub checked: bool,
}

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let tc = app.colors();
    let state = &mut app.device_info_state;

    // Lazy load device info
    if !state.checked {
        state.checked = true;
        state.cpu_model = get_cpu_model();
        if app.python_env_status.python_available {
            if let Some(ref py) = crate::services::python_env::resolved_python() {
                state.gpu_info = query_gpu_info(py);
            }
        }
    }

    page_header(ui, "设备信息", "查看系统设备与运行环境信息");

    let available = ui.available_size();
    let left_w = (available.x * 0.48).min(460.0).max(200.0);
    let right_w = (available.x - left_w - 16.0).max(200.0);

    ui.horizontal_top(|ui| {
        // ── 左侧：Python 环境 + 系统信息 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Python 环境")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(12.0);

                    info_grid(ui, "python_env", &tc, |ui| {
                        row(ui, "Python 状态:", &tc, |ui| {
                            if app.python_env_status.python_available {
                                ui.colored_label(AppleColors::SUCCESS, "可用");
                            } else {
                                ui.colored_label(AppleColors::DANGER, "不可用");
                            }
                        });
                        row(ui, "Python 版本:", &tc, |ui| {
                            ui.label(app.python_env_status.python_version.as_deref().unwrap_or("未安装"));
                        });
                        row(ui, "PyTorch 状态:", &tc, |ui| {
                            if app.python_env_status.torch_available {
                                ui.colored_label(AppleColors::SUCCESS, "已安装");
                            } else {
                                ui.colored_label(AppleColors::DANGER, "未安装");
                            }
                        });
                        row(ui, "PyTorch 版本:", &tc, |ui| {
                            ui.label(app.python_env_status.torch_version.as_deref().unwrap_or("未安装"));
                        });
                        row(ui, "CUDA 状态:", &tc, |ui| {
                            if app.python_env_status.cuda_available {
                                ui.colored_label(AppleColors::SUCCESS, "可用");
                            } else {
                                ui.colored_label(AppleColors::WARNING, "不可用");
                            }
                        });
                    });

                    if !app.python_env_status.python_available {
                        ui.add_space(8.0);
                        if ui.button("前往环境设置").clicked() {
                            app.route = crate::app::Route::Settings;
                        }
                    }
                });

                ui.add_space(12.0);

                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("系统信息")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(12.0);

                    info_grid(ui, "system_info", &tc, |ui| {
                        row(ui, "操作系统:", &tc, |ui| { ui.label(std::env::consts::OS); });
                        row(ui, "架构:", &tc, |ui| { ui.label(std::env::consts::ARCH); });
                        row(ui, "CPU 型号:", &tc, |ui| { ui.label(&state.cpu_model); });
                        row(ui, "逻辑核心数:", &tc, |ui| {
                            ui.label(format!("{}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)));
                        });
                        row(ui, "RustTools 版本:", &tc, |ui| { ui.label(env!("CARGO_PKG_VERSION")); });
                        row(ui, "egui 版本:", &tc, |ui| { ui.label("0.34"); });
                    });
                });
            },
        );

        ui.add_space(16.0);

        // ── 右侧：GPU 详情 + CPU 信息 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("GPU 信息")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(12.0);

                    if let Some(ref gpus) = state.gpu_info {
                        if gpus.is_empty() {
                            empty_state(ui, "", "未检测到 CUDA GPU", "系统将使用 CPU 进行推理和训练。", &tc);
                        } else {
                            for (i, gpu) in gpus.iter().enumerate() {
                                if i > 0 {
                                    ui.separator();
                                    ui.add_space(8.0);
                                }
                                ui.label(
                                    egui::RichText::new(format!("GPU {}", i))
                                        .size(13.0)
                                        .strong(),
                                );
                                ui.add_space(8.0);

                                info_grid(ui, &format!("gpu_{}", i), &tc, |ui| {
                                    row(ui, "型号:", &tc, |ui| { ui.label(&gpu.name); });
                                    row(ui, "显存总量:", &tc, |ui| {
                                        ui.label(format!("{:.1} GB", gpu.total_memory_mb as f64 / 1024.0));
                                    });
                                    row(ui, "显存使用:", &tc, |ui| {
                                        let used_pct = if gpu.total_memory_mb > 0 {
                                            gpu.used_memory_mb as f32 / gpu.total_memory_mb as f32
                                        } else {
                                            0.0
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{:.1} GB", gpu.used_memory_mb as f64 / 1024.0));
                                            ui.add(
                                                egui::ProgressBar::new(used_pct)
                                                    .desired_width(80.0)
                                                    .text(format!("{:.0}%", used_pct * 100.0)),
                                            );
                                        });
                                    });
                                    row(ui, "CUDA 版本:", &tc, |ui| { ui.label(&gpu.cuda_version); });
                                    row(ui, "cuDNN 版本:", &tc, |ui| { ui.label(&gpu.cudnn_version); });
                                });
                            }
                        }
                    } else if app.python_env_status.python_available {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.spinner();
                            ui.label("正在检测 GPU 信息...");
                            ui.add_space(20.0);
                        });
                    } else {
                        empty_state(
                            ui,
                            "",
                            "未检测到 CUDA",
                            "Python 环境未就绪，无法检测 GPU 详情。",
                            &tc,
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("如需使用 GPU 加速，请确保：")
                                .size(12.0)
                                .color(tc.text_secondary),
                        );
                        ui.label(
                            egui::RichText::new("• 已安装 NVIDIA 显卡驱动")
                                .size(11.0)
                                .color(tc.text_secondary),
                        );
                        ui.label(
                            egui::RichText::new("• 已安装 CUDA Toolkit")
                                .size(11.0)
                                .color(tc.text_secondary),
                        );
                        ui.label(
                            egui::RichText::new("• 安装环境时选择 GPU 版本 PyTorch")
                                .size(11.0)
                                .color(tc.text_secondary),
                        );
                    }
                });

                ui.add_space(12.0);

                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("CPU 信息")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(12.0);

                    info_grid(ui, "cpu_info", &tc, |ui| {
                        row(ui, "逻辑核心数:", &tc, |ui| {
                            ui.label(format!("{}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)));
                        });
                        row(ui, "计算模式:", &tc, |ui| {
                            if app.python_env_status.cuda_available {
                                ui.colored_label(AppleColors::SUCCESS, "GPU 加速");
                            } else {
                                ui.label("CPU 模式");
                            }
                        });
                    });
                });
            },
        );
    });
}

fn info_grid(ui: &mut egui::Ui, id: &str, _tc: &crate::theme::ThemeColors, add_rows: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(ui.id().with(id))
        .num_columns(2)
        .spacing([20.0, 10.0])
        .show(ui, add_rows);
}

fn row(ui: &mut egui::Ui, label: &str, tc: &crate::theme::ThemeColors, mut add_value: impl FnMut(&mut egui::Ui)) {
    ui.label(egui::RichText::new(label).color(tc.text_secondary));
    add_value(ui);
    ui.end_row();
}

fn empty_state(ui: &mut egui::Ui, _icon: &str, title: &str, desc: &str, tc: &crate::theme::ThemeColors) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        // 绘制芯片/设备轮廓图标
        let icon_size = 40.0;
        let icon_rect = ui.allocate_exact_size(egui::vec2(icon_size, icon_size), egui::Sense::hover()).1.rect;
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.5, tc.text_tertiary.gamma_multiply(0.5));
        let body = icon_rect.shrink(4.0);
        painter.rect_stroke(body, egui::CornerRadius::same(4), stroke, egui::StrokeKind::Inside);
        painter.circle_stroke(body.center(), body.width().min(body.height()) * 0.2, stroke);
        painter.line_segment([body.center() + egui::vec2(-6.0, 0.0), body.center() + egui::vec2(6.0, 0.0)], stroke);
        painter.line_segment([body.center() + egui::vec2(0.0, -6.0), body.center() + egui::vec2(0.0, 6.0)], stroke);

        ui.add_space(8.0);
        ui.label(egui::RichText::new(title).size(14.0).strong().color(tc.text));
        ui.label(egui::RichText::new(desc).color(tc.text_secondary));
        ui.add_space(8.0);
    });
}

fn get_cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if line.starts_with("model name") {
                    if let Some(idx) = line.find(':') {
                        return line[idx + 1..].trim().to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["cpu", "get", "name", "/value"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    if let Some(idx) = line.find('=') {
                        return line[idx + 1..].trim().to_string();
                    }
                }
            }
        }
    }
    "Unknown".to_string()
}

fn query_gpu_info(python: &str) -> Option<Vec<GpuInfo>> {
    let script = r#"
import sys, json
try:
    import torch
    gpus = []
    if torch.cuda.is_available():
        for i in range(torch.cuda.device_count()):
            props = torch.cuda.get_device_properties(i)
            mem_total = props.total_memory // (1024 * 1024)
            mem_alloc = torch.cuda.memory_allocated(i) // (1024 * 1024)
            gpus.append({
                'name': props.name,
                'total_memory_mb': mem_total,
                'used_memory_mb': mem_alloc,
                'cuda_version': torch.version.cuda or 'Unknown',
                'cudnn_version': str(torch.backends.cudnn.version()) if torch.backends.cudnn.is_available() else 'Unavailable',
            })
    print(json.dumps(gpus))
except Exception as e:
    print(json.dumps([]))
"#;

    let output = std::process::Command::new(python)
        .arg("-c")
        .arg(script)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).ok()
}
