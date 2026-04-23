use eframe::egui;
use crate::app::RustToolsApp;
use crate::theme::{AppleColors, compact_card_frame, page_header};

/// 训练页面状态
#[derive(Default)]
pub struct TrainingPageState {
    pub base_model_idx: usize,
    pub epochs: u32,
    pub batch_size: u32,
    pub image_size: u32,
    pub learning_rate: f32,
    pub optimizer_idx: usize,
    pub workers: u32,
    pub amp: bool,
    pub cache: bool,
    pub single_cls: bool,
    pub cos_lr: bool,
    pub warmup: f32,
    pub save_period: i32,
    #[allow(dead_code)]
    pub is_training: bool,
    pub log_messages: Vec<String>,
    pub current_epoch: u32,
    pub total_epochs: u32,
    pub progress: f32,
}

const BASE_MODELS: &[&str] = &["yolo11n.pt", "yolo11s.pt", "yolo11m.pt", "yolo11l.pt", "yolo11x.pt"];
const OPTIMIZERS: &[&str] = &["SGD", "Adam", "AdamW"];

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    page_header(ui, "模型训练", "配置训练参数并监控训练过程");

    // 检查项目
    if app.current_project.is_none() {
        show_no_project(ui, app);
        return;
    }

    // 自动填充训练参数：从项目配置中同步默认值
    if let Some(ref project) = app.current_project {
        if app.training_state.image_size == 0 {
            app.training_state.image_size = project.image_size as u32;
        }
        if app.training_state.epochs == 0 {
            app.training_state.epochs = 100;
        }
        if app.training_state.batch_size == 0 {
            app.training_state.batch_size = 16;
        }
        if app.training_state.learning_rate == 0.0 {
            app.training_state.learning_rate = 0.01;
        }
        // workers 默认保持 0（由 Default derive 自动设为 0）
        // Linux 下多进程数据加载器容易死锁，用户可手动调高
        if app.training_state.warmup == 0.0 {
            app.training_state.warmup = 3.0;
        }
        if app.training_state.save_period == 0 {
            app.training_state.save_period = -1;
        }
    }

    // 检查 Python 环境
    if !app.python_env_status.python_available {
        ui.horizontal(|ui| {
            let (warn_rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            let painter = ui.painter();
            let c = warn_rect.center();
            let r = 6.0;
            let p1 = c + egui::vec2(0.0, -r);
            let p2 = c + egui::vec2(r * 0.866, r * 0.5);
            let p3 = c + egui::vec2(-r * 0.866, r * 0.5);
            painter.line_segment([p1, p2], egui::Stroke::new(1.5, AppleColors::DANGER));
            painter.line_segment([p2, p3], egui::Stroke::new(1.5, AppleColors::DANGER));
            painter.line_segment([p3, p1], egui::Stroke::new(1.5, AppleColors::DANGER));
            painter.circle_filled(c + egui::vec2(0.0, r * 0.25), 1.0, AppleColors::DANGER);
            ui.label("Python 环境未就绪，无法启动训练。");
        });
        ui.add_space(12.0);
    }

    let available = ui.available_size();
    let left_w = (available.x * 0.38).min(420.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：训练配置 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("训练配置")
                            .size(14.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(12.0);

                    config_row(ui, "基础模型:", |ui| {
                        let model_name = BASE_MODELS.get(app.training_state.base_model_idx).unwrap_or(&BASE_MODELS[0]);
                        egui::ComboBox::from_id_salt("base_model")
                            .selected_text(*model_name)
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in BASE_MODELS.iter().enumerate() {
                                    ui.selectable_value(&mut app.training_state.base_model_idx, i, *name);
                                }
                            });
                    });
                    config_row(ui, "训练轮数:", |ui| {
                        ui.add(egui::DragValue::new(&mut app.training_state.epochs).speed(1).range(1..=1000));
                        ui.label("epochs");
                    });
                    config_row(ui, "批次大小:", |ui| {
                        ui.add(egui::DragValue::new(&mut app.training_state.batch_size).speed(1).range(1..=128));
                    });
                    config_row(ui, "图像尺寸:", |ui| {
                        ui.add(egui::DragValue::new(&mut app.training_state.image_size).speed(32).range(320..=1280));
                    });
                    config_row(ui, "初始学习率:", |ui| {
                        ui.add(egui::DragValue::new(&mut app.training_state.learning_rate).speed(0.001).range(0.0001..=0.1));
                    });
                    config_row(ui, "优化器:", |ui| {
                        let opt_name = OPTIMIZERS.get(app.training_state.optimizer_idx).unwrap_or(&OPTIMIZERS[0]);
                        egui::ComboBox::from_id_salt("optimizer")
                            .selected_text(*opt_name)
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in OPTIMIZERS.iter().enumerate() {
                                    ui.selectable_value(&mut app.training_state.optimizer_idx, i, *name);
                                }
                            });
                    });
                    config_row(ui, "计算设备:", |ui| {
                        let device = if app.python_env_status.cuda_available {
                            "GPU (CUDA)"
                        } else {
                            "CPU"
                        };
                        ui.label(device);
                    });
                    ui.horizontal(|ui| {
                        config_row(ui, "数据加载线程:", |ui| {
                            ui.add(egui::DragValue::new(&mut app.training_state.workers).speed(1).range(0..=32));
                        });
                        ui.label(
                            egui::RichText::new("(Linux 建议 0，避免多进程死锁)")
                                .size(11.0)
                                .color(AppleColors::TEXT_TERTIARY),
                        );
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.collapsing("高级选项", |ui| {
                        ui.checkbox(&mut app.training_state.amp, "自动混合精度 (AMP)");
                        ui.checkbox(&mut app.training_state.cache, "缓存数据集到内存");
                        ui.checkbox(&mut app.training_state.single_cls, "单类别模式");
                        ui.checkbox(&mut app.training_state.cos_lr, "余弦学习率调度");
                        ui.add_space(4.0);
                        config_row(ui, "预热轮数:", |ui| {
                            ui.add(egui::DragValue::new(&mut app.training_state.warmup).speed(0.5).range(0.0..=10.0));
                        });
                        config_row(ui, "保存周期:", |ui| {
                            ui.add(egui::DragValue::new(&mut app.training_state.save_period).speed(1).range(-1..=100));
                            ui.label("(-1=仅最后)");
                        });
                    });

                    ui.add_space(16.0);

                    // 控制按钮
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let can_train = app.python_env_status.python_available && app.current_project.is_some();
                        let is_training = app.current_training_id.is_some();

                        if !is_training {
                            let start_btn = egui::Button::new(
                                egui::RichText::new("开始训练").color(AppleColors::SURFACE).strong(),
                            )
                            .fill(AppleColors::SUCCESS)
                            .corner_radius(egui::CornerRadius::same(8));
                            if ui.add_sized([110.0, 36.0], start_btn).clicked() && can_train {
                                start_training(app);
                            }
                        } else {
                            if ui.add_sized([90.0, 36.0], egui::Button::new("暂停")).clicked() {
                                if let Some(ref id) = app.current_training_id {
                                    let _ = app.tokio_rt.block_on(app.trainer_service.pause_training(id));
                                    app.training_state.log_messages.push("训练已暂停".to_string());
                                }
                            }
                            if ui
                                .add_sized([90.0, 36.0], egui::Button::new("停止").fill(AppleColors::DANGER))
                                .clicked()
                            {
                                if let Some(ref id) = app.current_training_id {
                                    let _ = app.tokio_rt.block_on(app.trainer_service.stop_training(id));
                                    app.current_training_id = None;
                                    app.training_state.log_messages.push("训练已停止".to_string());
                                }
                            }
                        }
                    });
                });
            },
        );

        ui.add_space(8.0);

        // ── 右侧：训练监控 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 轮询训练状态
                let training_active = if let Some(ref id) = app.current_training_id {
                    // 获取 Python 端日志
                    let logs = app.tokio_rt.block_on(app.trainer_service.get_logs(id));
                    for msg in logs {
                        app.training_state.log_messages.push(msg);
                    }

                    if let Some(status) = app.tokio_rt.block_on(app.trainer_service.get_status(id)) {
                        app.training_state.current_epoch = status.epoch;
                        app.training_state.total_epochs = status.total_epochs;
                        app.training_state.progress = if status.total_epochs > 0 {
                            status.epoch as f32 / status.total_epochs as f32
                        } else {
                            0.0
                        };

                        if !status.running && status.error.is_none() {
                            app.current_training_id = None;
                            app.training_state.log_messages.push("训练完成！".to_string());
                        }

                        if let Some(ref err) = status.error {
                            app.current_training_id = None;
                            app.training_state.log_messages.push(format!("训练错误: {}", err));
                        }

                        status.running
                    } else {
                        false
                    }
                } else {
                    false
                };

                // 训练进度卡片
                compact_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("训练进度")
                                .size(14.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        if training_active {
                            ui.label(
                                egui::RichText::new("运行中")
                                    .size(11.0)
                                    .color(AppleColors::SUCCESS),
                            );
                        }
                    });
                    ui.add_space(12.0);

                    if training_active {
                        // 大数字 Epoch 显示
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{}", app.training_state.current_epoch))
                                        .size(36.0)
                                        .strong()
                                        .color(AppleColors::TEXT),
                                );
                                ui.label(
                                    egui::RichText::new(format!("/ {} epochs", app.training_state.total_epochs))
                                        .size(12.0)
                                        .color(AppleColors::TEXT_SECONDARY),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:.0}%", app.training_state.progress * 100.0))
                                        .size(24.0)
                                        .strong()
                                        .color(AppleColors::SUCCESS),
                                );
                            });
                        });
                        ui.add_space(8.0);

                        // 进度条
                        let bar_w = ui.available_width();
                        let bar_h = 8.0;
                        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(bar_rect, egui::CornerRadius::same(4), AppleColors::BG_DEEP);
                        let fill_w = bar_w * app.training_state.progress;
                        if fill_w > 0.0 {
                            let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_w, bar_h));
                            painter.rect_filled(fill_rect, egui::CornerRadius::same(4), AppleColors::SUCCESS);
                        }

                        ui.add_space(16.0);

                        // 指标卡片网格
                        ui.label(egui::RichText::new("指标概览").size(12.0).strong().color(AppleColors::TEXT));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let metrics = if let Some(ref tid) = app.current_training_id {
                                app.tokio_rt.block_on(app.trainer_service.get_status(tid)).map(|s| s.metrics)
                            } else { None }.unwrap_or_default();
                            metric_mini_card(ui, "Box Loss",
                                &format!("{:.3}", metrics.train_box_loss), AppleColors::PRIMARY);
                            metric_mini_card(ui, "Cls Loss",
                                &format!("{:.3}", metrics.train_cls_loss), AppleColors::WARNING);
                            metric_mini_card(ui, "mAP@50",
                                &format!("{:.3}", metrics.map50), AppleColors::SUCCESS);
                        });
                    } else {
                        // 未训练空状态
                        ui.vertical_centered(|ui| {
                            ui.add_space(24.0);
                            // 绘制图表轮廓
                            let icon_w = 80.0;
                            let icon_h = 50.0;
                            let icon_rect = ui.allocate_exact_size(egui::vec2(icon_w, icon_h), egui::Sense::hover()).1.rect;
                            let painter = ui.painter();
                            let stroke = egui::Stroke::new(1.5, AppleColors::TEXT_TERTIARY.gamma_multiply(0.5));
                            let chart = icon_rect.shrink(4.0);
                            painter.rect_stroke(chart, egui::CornerRadius::same(4), stroke, egui::StrokeKind::Inside);
                            let p1 = chart.min + egui::vec2(chart.width() * 0.1, chart.height() * 0.8);
                            let p2 = chart.min + egui::vec2(chart.width() * 0.3, chart.height() * 0.6);
                            let p3 = chart.min + egui::vec2(chart.width() * 0.5, chart.height() * 0.65);
                            let p4 = chart.min + egui::vec2(chart.width() * 0.75, chart.height() * 0.35);
                            let p5 = chart.min + egui::vec2(chart.width() * 0.9, chart.height() * 0.25);
                            painter.line_segment([p1, p2], stroke);
                            painter.line_segment([p2, p3], stroke);
                            painter.line_segment([p3, p4], stroke);
                            painter.line_segment([p4, p5], stroke);
                            painter.circle_filled(p5, 2.5, AppleColors::TEXT_TERTIARY.gamma_multiply(0.5));

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("未开始训练").size(14.0).strong().color(AppleColors::TEXT));
                            ui.label(egui::RichText::new("配置参数后点击「开始训练」").size(12.0).color(AppleColors::TEXT_SECONDARY));
                            ui.add_space(24.0);
                        });
                    }
                });

                ui.add_space(8.0);

                // 训练日志
                compact_card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("训练日志")
                            .size(14.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(4.0);

                    let log_h = ui.available_height() - 4.0;
                    egui::ScrollArea::vertical()
                        .max_height(log_h.max(60.0))
                        .show(ui, |ui| {
                            if app.training_state.log_messages.is_empty() {
                                ui.monospace(
                                    egui::RichText::new("等待训练开始...")
                                        .size(11.0)
                                        .color(AppleColors::TEXT_SECONDARY),
                                );
                            } else {
                                for msg in &app.training_state.log_messages {
                                    ui.monospace(egui::RichText::new(msg).size(11.0));
                                }
                            }
                        });
                });
            },
        );
    });
}

fn show_no_project(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let available = ui.available_size();
    ui.allocate_ui_with_layout(
        egui::vec2(available.x, available.y * 0.6),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(60.0);
            // 绘制趋势图轮廓图标
            let icon_size = 64.0;
            let icon_rect = ui.allocate_exact_size(egui::vec2(icon_size, icon_size), egui::Sense::hover()).1.rect;
            let painter = ui.painter();
            let stroke = egui::Stroke::new(2.0, AppleColors::TEXT_TERTIARY);
            let chart = icon_rect.shrink(8.0);
            // 外框
            painter.rect_stroke(chart, egui::CornerRadius::same(6), stroke, egui::StrokeKind::Inside);
            // 折线趋势
            let p1 = chart.min + egui::vec2(chart.width() * 0.15, chart.height() * 0.75);
            let p2 = chart.min + egui::vec2(chart.width() * 0.35, chart.height() * 0.55);
            let p3 = chart.min + egui::vec2(chart.width() * 0.55, chart.height() * 0.65);
            let p4 = chart.min + egui::vec2(chart.width() * 0.80, chart.height() * 0.30);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            // 终点圆点
            painter.circle_filled(p4, 3.0, AppleColors::TEXT_TERTIARY);

            ui.add_space(16.0);
            ui.label(egui::RichText::new("未打开项目").size(18.0).strong().color(AppleColors::TEXT));
            ui.label(egui::RichText::new("训练需要选择一个包含数据集的 YOLO 项目。").color(AppleColors::TEXT_SECONDARY));
            ui.add_space(16.0);
            let btn = egui::Button::new(egui::RichText::new("打开项目").color(AppleColors::SURFACE).strong())
                .fill(AppleColors::PRIMARY)
                .corner_radius(egui::CornerRadius::same(8));
            if ui.add_sized([100.0, 36.0], btn).clicked() {
                if let Some(path) = crate::ui::project::pick_folder_fallback() {
                    let result = crate::services::project::open_project(path.to_string_lossy().to_string());
                    if result.success {
                        if let Some(config) = result.data {
                            app.current_project = Some(config);
                        }
                    } else {
                        app.last_error = result.error.or_else(|| Some("打开项目失败".to_string()));
                    }
                }
            }
        },
    );
}

fn config_row(ui: &mut egui::Ui, label: &str, mut add_control: impl FnMut(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [90.0, 20.0],
            egui::Label::new(
                egui::RichText::new(label).color(AppleColors::TEXT_SECONDARY).size(13.0),
            ),
        );
        add_control(ui);
    });
    ui.add_space(4.0);
}

fn start_training(app: &mut RustToolsApp) {
    let state = &mut app.training_state;
    if let Some(ref project) = app.current_project {
        let request = crate::models::TrainingRequest {
            base_model: BASE_MODELS.get(state.base_model_idx).unwrap_or(&BASE_MODELS[0]).to_string(),
            epochs: state.epochs,
            batch_size: state.batch_size,
            image_size: state.image_size,
            device_id: if app.python_env_status.cuda_available { 0 } else { -1 },
            workers: state.workers,
            optimizer: OPTIMIZERS.get(state.optimizer_idx).unwrap_or(&OPTIMIZERS[0]).to_string(),
            lr0: state.learning_rate,
            lrf: 0.01,
            momentum: 0.937,
            weight_decay: 0.0005,
            warmup_epochs: state.warmup,
            warmup_bias_lr: 0.1,
            warmup_momentum: 0.8,
            hsv_h: 0.015,
            hsv_s: 0.7,
            hsv_v: 0.4,
            translate: 0.1,
            scale: 0.5,
            shear: 0.0,
            perspective: 0.0,
            flipud: 0.0,
            fliplr: 0.5,
            mosaic: 1.0,
            mixup: 0.0,
            copy_paste: 0.0,
            close_mosaic: 10,
            rect: false,
            cos_lr: state.cos_lr,
            single_cls: state.single_cls,
            amp: state.amp,
            save_period: state.save_period,
            cache: state.cache,
        };

        let project_path = project.path.clone();
        let trainer = &app.trainer_service;
        let result = app.tokio_rt.block_on(async {
            trainer.start_training(project_path, request).await
        });

        match result {
            Ok(training_id) => {
                app.current_training_id = Some(training_id.clone());
                state.log_messages.push(format!("训练已启动: {}", training_id));
                state.log_messages.push(format!(
                    "模型: {}, 轮数: {}, 批次: {}",
                    BASE_MODELS.get(state.base_model_idx).unwrap_or(&BASE_MODELS[0]),
                    state.epochs,
                    state.batch_size
                ));
            }
            Err(e) => {
                state.log_messages.push(format!("启动失败: {}", e));
            }
        }
    }
}

fn metric_mini_card(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(70.0, 48.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(8), color.gamma_multiply(0.08));
        painter.rect_stroke(rect, egui::CornerRadius::same(8), egui::Stroke::new(1.0, color.gamma_multiply(0.2)), egui::StrokeKind::Inside);
        painter.text(
            rect.center() + egui::vec2(0.0, -6.0),
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
            color,
        );
        painter.text(
            rect.center() + egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::new(9.0, egui::FontFamily::Proportional),
            AppleColors::TEXT_SECONDARY,
        );
    });
}
