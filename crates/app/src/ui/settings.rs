use eframe::egui;
use crate::app::RustToolsApp;
use crate::services::python_env::{get_env_status, UvManager};
use crate::theme::{AppleColors, card_frame, page_header};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone)]
pub enum InstallState {
    Idle,
    Installing,
    Success(String),
    Error(String),
}

#[derive(Debug)]
pub struct SettingsState {
    pub install_state: InstallState,
    pub progress_messages: Vec<String>,
    #[allow(dead_code)]
    pub install_tx: Option<Sender<InstallMessage>>,
    pub install_rx: Option<Receiver<InstallMessage>>,
}

#[derive(Debug, Clone)]
pub enum InstallMessage {
    Progress(String),
    Done(Result<(Option<String>, Option<String>), String>),
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            install_state: InstallState::Idle,
            progress_messages: Vec::new(),
            install_tx: None,
            install_rx: None,
        }
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let state = &mut app.settings_state;

    // 从接收器收集消息
    let messages: Vec<InstallMessage> = if let Some(ref rx) = state.install_rx {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    } else {
        Vec::new()
    };

    for msg in messages {
        match msg {
            InstallMessage::Progress(msg) => {
                state.progress_messages.push(msg);
                if state.progress_messages.len() > 50 {
                    state.progress_messages.remove(0);
                }
            }
            InstallMessage::Done(result) => {
                match result {
                    Ok((py_version, torch_version)) => {
                        state.install_state = InstallState::Success(format!(
                            "Python: {:?}, PyTorch: {:?}",
                            py_version, torch_version
                        ));
                        app.python_env_status = get_env_status();
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

    page_header(ui, "环境设置", "管理 Python 环境与依赖");

    let available = ui.available_size();
    let left_w = (available.x * 0.55).min(500.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：Python 环境 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Python 环境")
                                .size(14.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if matches!(state.install_state, InstallState::Idle | InstallState::Error(_))
                            {
                                if ui.button("一键安装").clicked() {
                                    start_installation(state);
                                }
                            }
                        });
                    });
                    ui.add_space(12.0);

                    egui::Grid::new("python_env_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Python:").color(AppleColors::TEXT_SECONDARY));
                            status_badge(ui, app.python_env_status.python_available,
                                app.python_env_status.python_version.as_deref().unwrap_or("未找到"));
                            ui.end_row();

                            ui.label(egui::RichText::new("PyTorch:").color(AppleColors::TEXT_SECONDARY));
                            status_badge(ui, app.python_env_status.torch_available,
                                app.python_env_status.torch_version.as_deref().unwrap_or("未安装"));
                            ui.end_row();

                            ui.label(egui::RichText::new("CUDA:").color(AppleColors::TEXT_SECONDARY));
                            status_badge(ui, app.python_env_status.cuda_available,
                                if app.python_env_status.cuda_available { "可用" } else { "不可用 / CPU 模式" });
                            ui.end_row();

                            if let Some(ref env_name) = app.python_env_status.conda_env_name {
                                ui.label(egui::RichText::new("Conda 环境:").color(AppleColors::TEXT_SECONDARY));
                                ui.label(env_name);
                                ui.end_row();
                            }
                        });

                    if let Some(ref error) = app.python_env_status.detection_error {
                        ui.add_space(8.0);
                        ui.colored_label(AppleColors::DANGER, format!("! {}", error));
                    }

                    match &state.install_state {
                        InstallState::Installing => {
                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("正在安装中...").strong());
                            });
                            ui.add(egui::ProgressBar::new(0.5).animate(true));

                            if !state.progress_messages.is_empty() {
                                ui.add_space(8.0);
                                egui::ScrollArea::vertical()
                                    .max_height(120.0)
                                    .show(ui, |ui| {
                                        for msg in &state.progress_messages {
                                            ui.monospace(
                                                egui::RichText::new(msg).size(11.0),
                                            );
                                        }
                                    });
                            }
                        }
                        InstallState::Success(ref details) => {
                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                ui.colored_label(AppleColors::SUCCESS, "安装完成");
                                ui.label(details);
                            });
                            if ui.button("清除日志").clicked() {
                                state.install_state = InstallState::Idle;
                                state.progress_messages.clear();
                            }
                        }
                        InstallState::Error(ref err) => {
                            ui.add_space(16.0);
                            ui.colored_label(AppleColors::DANGER, format!("错误: {}", err));
                            if ui.button("重试").clicked() {
                                state.install_state = InstallState::Idle;
                                state.progress_messages.clear();
                            }
                        }
                        _ => {}
                    }
                });

                ui.add_space(12.0);

                ui.collapsing("环境安装说明", |ui| {
                    ui.label("一键安装将自动执行以下步骤：");
                    ui.add_space(4.0);
                    ui.label("1. 检测并安装 uv 包管理器（如未安装）");
                    ui.label("2. 创建 Python 虚拟环境 ~/.rusttools/yolo-env");
                    ui.label("3. 根据 CUDA 可用性选择 PyTorch 版本");
                    ui.label("4. 安装 ultralytics、onnxruntime 等依赖");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "注意：首次安装可能需要 5-10 分钟，取决于网络速度。",
                        )
                        .color(AppleColors::TEXT_SECONDARY),
                    );
                });
            },
        );

        ui.add_space(16.0);

        // ── 右侧：关于 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("关于")
                            .size(14.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(12.0);
                    ui.label("RustTools - YOLO 目标检测工具箱");
                    ui.label(format!("版本: {}", env!("CARGO_PKG_VERSION")));
                    ui.hyperlink_to("GitHub 文档", "https://github.com/RustTools");
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
    state
        .progress_messages
        .push("开始配置 Python 环境...".to_string());

    let (tx, rx): (Sender<InstallMessage>, Receiver<InstallMessage>) = channel();
    let tx_progress = tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("创建 tokio 运行时失败");

        rt.block_on(async {
            let manager = UvManager::new();

            if manager.uv_path().is_none() {
                let _ = tx_progress.send(InstallMessage::Progress(
                    "正在安装 uv 包管理器...".to_string(),
                ));
                match UvManager::install_uv().await {
                    Ok(path) => {
                        let _ = tx_progress.send(InstallMessage::Progress(format!(
                            "uv 已安装至: {:?}",
                            path
                        )));
                    }
                    Err(e) => {
                        let _ = tx_progress.send(InstallMessage::Done(Err(e)));
                        return;
                    }
                }
            }

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

            let _ = tx_progress.send(InstallMessage::Progress(
                "正在安装 Python 依赖（可能需要几分钟）...".to_string(),
            ));

            let progress_cb = |msg: String| {
                let _ = tx_progress.send(InstallMessage::Progress(msg));
            };

            match manager.install_deps(progress_cb).await {
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
