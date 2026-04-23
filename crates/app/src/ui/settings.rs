use eframe::egui;
use crate::app::RustToolsApp;
use crate::services::env::{EnvReport, generate_env_report};
use crate::services::python_env::{get_env_status, UvManager, MirrorSource, InstallPlan};
use crate::theme::{AppleColors, card_frame, page_header};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone)]
pub enum InstallState {
    Idle,
    #[allow(dead_code)]
    Detecting,
    Installing,
    Success(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum InstallMessage {
    Progress(String),
    Done(Result<(Option<String>, Option<String>), String>),
}

#[derive(Debug)]
pub struct SettingsState {
    pub install_state: InstallState,
    pub progress_messages: Vec<String>,
    pub env_report: Option<EnvReport>,
    pub selected_mirror: MirrorSource,
    pub install_plan: Option<InstallPlan>,
    #[allow(dead_code)]
    pub install_tx: Option<Sender<InstallMessage>>,
    pub install_rx: Option<Receiver<InstallMessage>>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            install_state: InstallState::Idle,
            progress_messages: Vec::new(),
            env_report: None,
            selected_mirror: MirrorSource::Tsinghua,
            install_plan: None,
            install_tx: None,
            install_rx: None,
        }
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let state = &mut app.settings_state;

    // ── 收集安装线程消息 ──
    let messages: Vec<InstallMessage> = if let Some(ref rx) = state.install_rx {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    } else {
        Vec::new()
    };
    for msg in messages {
        match msg {
            InstallMessage::Progress(msg) => {
                state.progress_messages.push(msg);
                if state.progress_messages.len() > 100 {
                    state.progress_messages.remove(0);
                }
            }
            InstallMessage::Done(result) => {
                match result {
                    Ok((py_version, torch_version)) => {
                        state.install_state = InstallState::Success(format!(
                            "Python: {:?}, PyTorch: {:?}", py_version, torch_version
                        ));
                        app.python_env_status = get_env_status();
                        state.env_report = Some(generate_env_report());
                    }
                    Err(e) => {
                        state.install_state = InstallState::Error(e);
                    }
                }
                state.install_rx = None;
                state.install_tx = None;
            }
        }
    }

    page_header(ui, "环境设置", "环境检测、安装与配置");

    let available = ui.available_size();
    let left_w = (available.x * 0.55).min(520.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：环境检测与安装 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 环境检测卡片
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("环境检测").size(14.0).strong().color(AppleColors::TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 刷新检测").clicked() {
                                state.env_report = Some(generate_env_report());
                                state.install_plan = None;
                            }
                        });
                    });
                    ui.add_space(12.0);

                    // 懒加载环境报告
                    let report = state.env_report.get_or_insert_with(generate_env_report);

                    // 系统信息
                    ui.label(egui::RichText::new("系统信息").size(12.0).strong().color(AppleColors::TEXT));
                    ui.add_space(4.0);
                    egui::Grid::new("sys_info_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("操作系统:").color(AppleColors::TEXT_SECONDARY));
                        ui.label(format!("{} {} ({})", report.system.os, report.system.arch, report.system.os_version.as_deref().unwrap_or("未知")));
                        ui.end_row();

                        ui.label(egui::RichText::new("CPU 核心:").color(AppleColors::TEXT_SECONDARY));
                        ui.label(format!("{}", report.system.cpu_cores));
                        ui.end_row();

                        ui.label(egui::RichText::new("内存:").color(AppleColors::TEXT_SECONDARY));
                        ui.label(format!("{} MB", report.system.total_memory_mb));
                        ui.end_row();
                    });
                    ui.add_space(12.0);

                    // GPU / CUDA 信息
                    ui.label(egui::RichText::new("GPU / CUDA").size(12.0).strong().color(AppleColors::TEXT));
                    ui.add_space(4.0);
                    if report.cuda.available {
                        egui::Grid::new("cuda_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
                            ui.label(egui::RichText::new("CUDA 版本:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(report.cuda.runtime_version.as_deref().unwrap_or("未知"));
                            ui.end_row();
                            ui.label(egui::RichText::new("驱动版本:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(report.cuda.driver_version.as_deref().unwrap_or("未知"));
                            ui.end_row();
                            for (i, gpu) in report.cuda.gpus.iter().enumerate() {
                                ui.label(egui::RichText::new(format!("GPU {}:", i)).color(AppleColors::TEXT_SECONDARY));
                                ui.label(format!("{} ({:.0} MB)", gpu.name, gpu.memory_mb));
                                ui.end_row();
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.colored_label(AppleColors::TEXT_SECONDARY, "未检测到 NVIDIA GPU / CUDA");
                            if report.system.os == crate::services::env::OsType::MacOS {
                                ui.colored_label(AppleColors::TEXT_SECONDARY, "（macOS 使用 Apple Silicon / CPU 模式）");
                            }
                        });
                    }
                    ui.add_space(12.0);

                    // Python 环境状态
                    ui.label(egui::RichText::new("Python 环境").size(12.0).strong().color(AppleColors::TEXT));
                    ui.add_space(4.0);
                    egui::Grid::new("py_env_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("uv 包管理器:").color(AppleColors::TEXT_SECONDARY));
                        status_badge(ui, report.uv_installed, report.uv_version.as_deref().unwrap_or("未安装"));
                        ui.end_row();
                        ui.label(egui::RichText::new("Python:").color(AppleColors::TEXT_SECONDARY));
                        status_badge(ui, report.python_installed, report.python_version.as_deref().unwrap_or("未安装"));
                        ui.end_row();
                        ui.label(egui::RichText::new("PyTorch:").color(AppleColors::TEXT_SECONDARY));
                        let torch_label = if report.torch_available {
                            if report.torch_cuda { "已安装 (GPU)" } else { "已安装 (CPU)" }
                        } else { "未安装" };
                        status_badge(ui, report.torch_available, torch_label);
                        ui.end_row();
                        ui.label(egui::RichText::new("ONNX Runtime:").color(AppleColors::TEXT_SECONDARY));
                        let ort_label = if report.ort_available {
                            if report.ort_cuda { "已安装 (GPU)" } else { "已安装 (CPU)" }
                        } else { "未安装" };
                        status_badge(ui, report.ort_available, ort_label);
                        ui.end_row();
                    });
                });

                ui.add_space(12.0);

                // 安装配置卡片
                card_frame().show(ui, |ui| {
                    ui.label(egui::RichText::new("安装配置").size(14.0).strong().color(AppleColors::TEXT));
                    ui.add_space(12.0);

                    // 镜像源选择
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("PyPI 镜像源:").color(AppleColors::TEXT_SECONDARY));
                        egui::ComboBox::from_id_salt("mirror_select")
                            .selected_text(state.selected_mirror.label())
                            .width(160.0)
                            .show_ui(ui, |ui| {
                                for m in [MirrorSource::Tsinghua, MirrorSource::Aliyun, MirrorSource::USTC, MirrorSource::Default] {
                                    ui.selectable_value(&mut state.selected_mirror, m, m.label());
                                }
                            });
                    });
                    ui.add_space(8.0);

                    // 生成并显示安装方案
                    let plan = state.install_plan.get_or_insert_with(|| {
                        UvManager::generate_install_plan(state.selected_mirror)
                    });

                    ui.label(egui::RichText::new("检测到的安装方案:").color(AppleColors::TEXT_SECONDARY));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&plan.description).strong());
                    if let Some(ref warning) = plan.warning {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.colored_label(AppleColors::WARNING, "⚠ ");
                            ui.colored_label(AppleColors::WARNING, warning);
                        });
                    }
                    ui.add_space(8.0);
                    ui.label(format!("Python 版本: {} | 包数量: {}", plan.python_version, plan.packages.len()));

                    // 安装按钮
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let can_install = matches!(state.install_state, InstallState::Idle | InstallState::Error(_));
                        let btn_text = if can_install { "🚀 一键安装环境" } else { "安装中..." };
                        let btn = egui::Button::new(
                            egui::RichText::new(btn_text).color(AppleColors::SURFACE).strong(),
                        )
                        .fill(if can_install { AppleColors::PRIMARY } else { AppleColors::TEXT_SECONDARY })
                        .corner_radius(egui::CornerRadius::same(8));
                        if ui.add_sized([ui.available_width(), 40.0], btn).clicked() && can_install {
                            // 配置镜像
                            let _ = UvManager::configure_mirror(state.selected_mirror);
                            // 重新生成方案（镜像可能已改变）
                            state.install_plan = Some(UvManager::generate_install_plan(state.selected_mirror));
                            start_installation(state);
                        }
                    });

                    // 安装进度
                    match &state.install_state {
                        InstallState::Installing => {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("正在安装...").strong());
                            });
                            ui.add(egui::ProgressBar::new(0.5).animate(true).show_percentage());
                            if !state.progress_messages.is_empty() {
                                ui.add_space(8.0);
                                egui::ScrollArea::vertical()
                                    .max_height(120.0)
                                    .id_salt("install_log")
                                    .show(ui, |ui| {
                                        for msg in &state.progress_messages {
                                            ui.monospace(egui::RichText::new(msg).size(11.0).color(AppleColors::TEXT_SECONDARY));
                                        }
                                    });
                            }
                        }
                        InstallState::Success(ref details) => {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.colored_label(AppleColors::SUCCESS, "✓ 安装完成");
                                ui.label(egui::RichText::new(details).size(12.0));
                            });
                            if ui.button("清除日志").clicked() {
                                state.install_state = InstallState::Idle;
                                state.progress_messages.clear();
                            }
                        }
                        InstallState::Error(ref err) => {
                            ui.add_space(12.0);
                            ui.colored_label(AppleColors::DANGER, format!("✗ 错误: {}", err));
                            if ui.button("重试").clicked() {
                                state.install_state = InstallState::Idle;
                                state.progress_messages.clear();
                            }
                        }
                        _ => {}
                    }
                });
            },
        );

        ui.add_space(16.0);

        // ── 右侧：关于与帮助 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.label(egui::RichText::new("关于").size(14.0).strong().color(AppleColors::TEXT));
                    ui.add_space(12.0);
                    ui.label("RustTools - AI 目标检测工具箱");
                    ui.label(format!("版本: {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.hyperlink_to("GitHub", "https://github.com/go-enols/RustTools");
                });

                ui.add_space(12.0);

                card_frame().show(ui, |ui| {
                    ui.label(egui::RichText::new("安装说明").size(14.0).strong().color(AppleColors::TEXT));
                    ui.add_space(12.0);
                    ui.label("一键安装将自动完成：");
                    ui.add_space(4.0);
                    ui.label("1. 检测系统环境（OS、CUDA、GPU）");
                    ui.label("2. 安装 uv 包管理器（如未安装）");
                    ui.label("3. 创建 Python 虚拟环境");
                    ui.label("4. 根据 CUDA 版本安装 PyTorch（CPU/GPU）");
                    ui.label("5. 安装 ONNX Runtime 等推理依赖");
                    ui.add_space(8.0);
                    ui.colored_label(AppleColors::TEXT_SECONDARY, "首次安装约需 3-10 分钟，取决于网络速度。建议先选择国内镜像源以加速下载。");
                    ui.add_space(8.0);
                    ui.colored_label(AppleColors::TEXT_SECONDARY, "如需 CUDA 加速，请确保 NVIDIA 驱动已安装且版本 >= 525（支持 CUDA 12）。");
                });
            },
        );
    });
}

fn status_badge(ui: &mut egui::Ui, ok: bool, text: &str) {
    let (color, bg) = if ok {
        (AppleColors::SUCCESS, AppleColors::SUCCESS.gamma_multiply(0.12))
    } else {
        (AppleColors::DANGER, AppleColors::DANGER.gamma_multiply(0.12))
    };

    let response = ui.allocate_response(
        egui::vec2(ui.available_width().min(160.0), 24.0),
        egui::Sense::hover(),
    );

    ui.painter()
        .rect_filled(response.rect, egui::CornerRadius::same(6), bg);
    ui.painter().text(
        response.rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
        color,
    );
}

fn start_installation(state: &mut SettingsState) {
    state.install_state = InstallState::Installing;
    state.progress_messages.clear();
    state.progress_messages.push("开始配置 Python 环境...".to_string());

    let plan = state.install_plan.clone().unwrap_or_else(|| {
        UvManager::generate_install_plan(state.selected_mirror)
    });

    let (tx, rx): (Sender<InstallMessage>, Receiver<InstallMessage>) = channel();
    let tx_progress = tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("创建 tokio 运行时失败");

        rt.block_on(async {
            let manager = UvManager::new();

            // 1. 安装 uv
            if manager.uv_path().is_none() {
                let _ = tx_progress.send(InstallMessage::Progress(
                    "正在安装 uv 包管理器...".to_string(),
                ));
                match UvManager::install_uv().await {
                    Ok(path) => {
                        let _ = tx_progress.send(InstallMessage::Progress(format!(
                            "uv 已安装: {:?}", path
                        )));
                    }
                    Err(e) => {
                        let _ = tx_progress.send(InstallMessage::Done(Err(e)));
                        return;
                    }
                }
            }

            // 2. 创建虚拟环境
            if !manager.venv_exists() {
                let _ = tx_progress.send(InstallMessage::Progress(
                    "正在创建 Python 虚拟环境...".to_string(),
                ));
                if let Err(e) = manager.create_venv().await {
                    let _ = tx_progress.send(InstallMessage::Done(Err(e)));
                    return;
                }
                let _ = tx_progress.send(InstallMessage::Progress(
                    "虚拟环境创建完成".to_string(),
                ));
            }

            // 3. 按方案安装依赖
            let _ = tx_progress.send(InstallMessage::Progress(
                format!("按方案安装: {}...", plan.description),
            ));

            let progress_cb = |msg: String| {
                let _ = tx_progress.send(InstallMessage::Progress(msg));
            };

            match manager.install_with_plan(&plan, progress_cb).await {
                Ok(()) => {
                    let python_version = crate::services::python_env::check_python();
                    let torch_version = crate::services::python_env::check_torch();
                    let _ = tx_progress.send(InstallMessage::Done(Ok((
                        python_version,
                        torch_version,
                    ))));
                }
                Err(e) => {
                    let _ = tx_progress.send(InstallMessage::Done(Err(e)));
                }
            }
        });
    });

    state.install_tx = Some(tx);
    state.install_rx = Some(rx);
}
