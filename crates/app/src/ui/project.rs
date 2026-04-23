use eframe::egui;
use crate::app::{RustToolsApp, Route};
use crate::services::project::{create_project, open_project};
use crate::models::{DatasetPaths, ProjectConfig};
use crate::theme::{AppleColors, card_frame, page_header_with_action};

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let mut goto_annotation = false;
    let mut goto_training = false;

    // ── 显示最近一次错误消息 ──
    if let Some(ref err) = app.last_error {
        let err_frame = egui::Frame::new()
            .fill(crate::theme::AppleColors::DANGER.gamma_multiply(0.08))
            .stroke(egui::Stroke::new(1.0, crate::theme::AppleColors::DANGER.gamma_multiply(0.3)))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12));
        err_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // 警告三角图标
                let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                let c = icon_rect.center();
                let r = icon_rect.width() * 0.4;
                let p1 = c + egui::vec2(0.0, -r);
                let p2 = c + egui::vec2(-r * 0.866, r * 0.5);
                let p3 = c + egui::vec2(r * 0.866, r * 0.5);
                ui.painter().line_segment([p1, p2], egui::Stroke::new(2.0, crate::theme::AppleColors::DANGER));
                ui.painter().line_segment([p2, p3], egui::Stroke::new(2.0, crate::theme::AppleColors::DANGER));
                ui.painter().line_segment([p3, p1], egui::Stroke::new(2.0, crate::theme::AppleColors::DANGER));
                ui.painter().circle_filled(c + egui::vec2(0.0, r * 0.2), 1.5, crate::theme::AppleColors::DANGER);
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(err)
                        .color(crate::theme::AppleColors::DANGER)
                        .size(13.0),
                );
            });
        });
        ui.add_space(12.0);
        // 消费后清空，避免下一帧重复显示
        app.last_error = None;
    }

    page_header_with_action(
        ui,
        "项目管理",
        "创建和管理 YOLO 项目",
        "新建项目",
        || new_project_dialog(app),
    );

    if let Some(project) = app.current_project.clone() {
        show_project_detail(ui, &project, &mut goto_annotation, &mut goto_training);
    } else {
        show_empty_state(ui, app);
    }

    if goto_annotation {
        app.route = Route::Annotation;
    }
    if goto_training {
        app.route = Route::Training;
    }
}

fn show_project_detail(ui: &mut egui::Ui, project: &ProjectConfig, goto_annotation: &mut bool, goto_training: &mut bool) {
    let scan = scan_project_contents(project);
    let available = ui.available_size();
    let left_w = (available.x * 0.55).min(520.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：项目信息 + 统计 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 项目信息
                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("项目信息")
                            .size(14.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(12.0);

                    egui::Grid::new("project_info_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("项目名称:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(egui::RichText::new(&project.name).strong());
                            ui.end_row();

                            ui.label(egui::RichText::new("YOLO 版本:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(&project.yolo_version);
                            ui.end_row();

                            ui.label(egui::RichText::new("图像尺寸:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(format!("{}x{}", project.image_size, project.image_size));
                            ui.end_row();

                            ui.label(egui::RichText::new("训练/验证划分:").color(AppleColors::TEXT_SECONDARY));
                            ui.label(format!("{:.0}% / {:.0}%", project.train_split * 100.0, project.val_split * 100.0));
                            ui.end_row();
                        });

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let annot_btn = egui::Button::new(
                            egui::RichText::new("图像标注").color(AppleColors::SURFACE).strong(),
                        )
                        .fill(AppleColors::PINK)
                        .corner_radius(egui::CornerRadius::same(8));
                        if ui.add_sized([100.0, 36.0], annot_btn).clicked() {
                            *goto_annotation = true;
                        }
                        let train_btn = egui::Button::new(
                            egui::RichText::new("开始训练").color(AppleColors::SURFACE).strong(),
                        )
                        .fill(AppleColors::SUCCESS)
                        .corner_radius(egui::CornerRadius::same(8));
                        if ui.add_sized([100.0, 36.0], train_btn).clicked() {
                            *goto_training = true;
                        }
                    });
                });

                ui.add_space(12.0);

                // 数据集统计
                card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("数据集统计")
                            .size(14.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        stat_card(ui, "训练图像", &format!("{}", scan.train_images), AppleColors::PRIMARY);
                        stat_card(ui, "验证图像", &format!("{}", scan.val_images), AppleColors::WARNING);
                        stat_card(ui, "标注总数", &format!("{}", scan.total_annotations), AppleColors::SUCCESS);
                        stat_card(ui, "模型文件", &format!("{}", scan.model_count), AppleColors::PURPLE);
                        stat_card(ui, "训练记录", &format!("{}", scan.run_count), AppleColors::TEAL);
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("数据集路径")
                            .size(12.0)
                            .strong()
                            .color(AppleColors::TEXT),
                    );
                    ui.add_space(6.0);
                    egui::Grid::new("dataset_paths_grid")
                        .num_columns(2)
                        .spacing([16.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("训练图像:").color(AppleColors::TEXT_SECONDARY).size(11.0));
                            ui.monospace(egui::RichText::new(&project.images.train).size(11.0));
                            ui.end_row();
                            ui.label(egui::RichText::new("验证图像:").color(AppleColors::TEXT_SECONDARY).size(11.0));
                            ui.monospace(egui::RichText::new(&project.images.val).size(11.0));
                            ui.end_row();
                            ui.label(egui::RichText::new("训练标注:").color(AppleColors::TEXT_SECONDARY).size(11.0));
                            ui.monospace(egui::RichText::new(&project.labels.train).size(11.0));
                            ui.end_row();
                            ui.label(egui::RichText::new("验证标注:").color(AppleColors::TEXT_SECONDARY).size(11.0));
                            ui.monospace(egui::RichText::new(&project.labels.val).size(11.0));
                            ui.end_row();
                        });
                });
            },
        );

        ui.add_space(16.0);

        // ── 右侧：类别列表 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("目标类别")
                                .size(14.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        ui.label(
                            egui::RichText::new(format!("{} 个", project.classes.len()))
                                .size(12.0)
                                .color(AppleColors::TEXT_SECONDARY),
                        );
                    });
                    ui.add_space(12.0);

                    let chip_colors = [
                        AppleColors::PRIMARY,
                        AppleColors::SUCCESS,
                        AppleColors::WARNING,
                        AppleColors::DANGER,
                        AppleColors::PURPLE,
                        AppleColors::TEAL,
                        AppleColors::PINK,
                        AppleColors::INDIGO,
                    ];

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                        for (i, class) in project.classes.iter().enumerate() {
                            let color = chip_colors[i % chip_colors.len()];
                            let label = format!("{}: {}", i, class);
                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(80.0, 28.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(6),
                                color.gamma_multiply(0.12),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                egui::CornerRadius::same(6),
                                egui::Stroke::new(1.0, color.gamma_multiply(0.3)),
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &label,
                                egui::FontId::new(12.0, egui::FontFamily::Proportional),
                                color,
                            );
                        }
                    });

                    // 模型文件列表
                    if scan.model_count > 0 {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("模型文件")
                                .size(13.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        ui.add_space(6.0);
                        for model in &scan.models {
                            ui.horizontal(|ui| {
                                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 3.0, AppleColors::PURPLE);
                                ui.label(egui::RichText::new(model).size(11.0).color(AppleColors::TEXT_SECONDARY));
                            });
                        }
                        if scan.model_count > scan.models.len() {
                            ui.label(
                                egui::RichText::new(format!("... 还有 {} 个", scan.model_count - scan.models.len()))
                                    .size(11.0)
                                    .color(AppleColors::TEXT_TERTIARY),
                            );
                        }
                    }

                    // 训练记录列表
                    if scan.run_count > 0 {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("训练记录")
                                .size(13.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        ui.add_space(6.0);
                        for run in &scan.runs {
                            ui.horizontal(|ui| {
                                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 3.0, AppleColors::TEAL);
                                ui.label(egui::RichText::new(run).size(11.0).color(AppleColors::TEXT_SECONDARY));
                            });
                        }
                        if scan.run_count > scan.runs.len() {
                            ui.label(
                                egui::RichText::new(format!("... 还有 {} 个", scan.run_count - scan.runs.len()))
                                    .size(11.0)
                                    .color(AppleColors::TEXT_TERTIARY),
                            );
                        }
                    }
                });
            },
        );
    });
}

fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(80.0, 56.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(8), color.gamma_multiply(0.08));
        painter.rect_stroke(rect, egui::CornerRadius::same(8), egui::Stroke::new(1.0, color.gamma_multiply(0.2)), egui::StrokeKind::Inside);
        painter.text(
            rect.center() + egui::vec2(0.0, -6.0),
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::new(16.0, egui::FontFamily::Proportional),
            color,
        );
        painter.text(
            rect.center() + egui::vec2(0.0, 12.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::new(10.0, egui::FontFamily::Proportional),
            AppleColors::TEXT_SECONDARY,
        );
    });
}

#[derive(Default)]
struct ProjectScanResult {
    train_images: usize,
    val_images: usize,
    total_annotations: usize,
    models: Vec<String>,
    model_count: usize,
    runs: Vec<String>,
    run_count: usize,
}

fn scan_project_contents(project: &ProjectConfig) -> ProjectScanResult {
    let mut result = ProjectScanResult::default();
    let base = std::path::Path::new(&project.path);

    // 统计训练图像
    let train_img_dir = base.join(&project.images.train);
    if let Ok(entries) = std::fs::read_dir(&train_img_dir) {
        result.train_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

    // 统计验证图像
    let val_img_dir = base.join(&project.images.val);
    if let Ok(entries) = std::fs::read_dir(&val_img_dir) {
        result.val_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

    // 统计标注文件
    let train_label_dir = base.join(&project.labels.train);
    if let Ok(entries) = std::fs::read_dir(&train_label_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                result.total_annotations += content.lines().filter(|l| !l.trim().is_empty()).count();
            }
        }
    }
    let val_label_dir = base.join(&project.labels.val);
    if let Ok(entries) = std::fs::read_dir(&val_label_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                result.total_annotations += content.lines().filter(|l| !l.trim().is_empty()).count();
            }
        }
    }

    // 扫描模型文件
    let models_dir = base.join("models");
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().eq_ignore_ascii_case("pt") {
                    result.model_count += 1;
                    if result.models.len() < 5 {
                        if let Some(name) = path.file_name() {
                            result.models.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    // 扫描训练结果
    let runs_dir = base.join("runs");
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.run_count += 1;
                if result.runs.len() < 5 {
                    if let Some(name) = path.file_name() {
                        result.runs.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    result
}

fn show_empty_state(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let available = ui.available_size();
    ui.allocate_ui_with_layout(
        egui::vec2(available.x, available.y * 0.7),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(80.0);

            // 绘制柔和的文件夹轮廓图标
            let icon_size = 64.0;
            let icon_rect = ui.allocate_exact_size(egui::vec2(icon_size, icon_size), egui::Sense::hover()).1.rect;
            let painter = ui.painter();
            let stroke = egui::Stroke::new(2.0, AppleColors::TEXT_TERTIARY);
            let folder_body = icon_rect.shrink(4.0);
            // 主体
            painter.rect_stroke(folder_body, egui::CornerRadius::same(8), stroke, egui::StrokeKind::Inside);
            // 标签页
            let tab_rect = egui::Rect::from_min_size(
                folder_body.min + egui::vec2(8.0, -6.0),
                egui::vec2(24.0, 12.0),
            );
            painter.rect_stroke(tab_rect, egui::CornerRadius::same(4), stroke, egui::StrokeKind::Inside);

            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("暂无项目")
                    .size(18.0)
                    .strong()
                    .color(AppleColors::TEXT),
            );
            ui.label(
                egui::RichText::new("点击下方按钮新建或打开一个项目")
                    .color(AppleColors::TEXT_SECONDARY),
            );
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                if ui.button("新建项目").clicked() {
                    new_project_dialog(app);
                }
                if ui.button("打开项目").clicked() {
                    open_project_dialog(app);
                }
            });
        },
    );
}

/// 尝试用 rfd 打开文件夹选择器，若 portal/GTK 均失败则 fallback 到 zenity
pub fn pick_folder_fallback() -> Option<std::path::PathBuf> {
    // 1. 优先 rfd（支持 portal + gtk3）
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        return Some(path);
    }
    eprintln!("[rfd] pick_folder returned None, trying zenity fallback...");

    // 2. Fallback: zenity
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let output = Command::new("zenity")
            .args(["--file-selection", "--directory", "--title=选择项目文件夹"])
            .output()
            .ok()?;
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(std::path::PathBuf::from(path_str));
            }
        }
    }
    None
}

fn new_project_dialog(app: &mut RustToolsApp) {
    if let Some(path) = pick_folder_fallback() {
        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();

        let config = ProjectConfig {
            name: project_name.clone(),
            path: path.to_string_lossy().to_string(),
            yolo_version: "yolo11".to_string(),
            classes: vec!["object".to_string()],
            train_split: 0.8,
            val_split: 0.2,
            image_size: 640,
            description: Some("由 RustTools 创建".to_string()),
            images: DatasetPaths {
                train: "images/train".to_string(),
                val: "images/val".to_string(),
            },
            labels: DatasetPaths {
                train: "labels/train".to_string(),
                val: "labels/val".to_string(),
            },
        };

        let result = create_project(config.clone());
        if result.success {
            app.current_project = Some(config);
            eprintln!("[project] created project at {}", path.display());
        } else {
            app.last_error = result.error.or_else(|| Some("创建项目失败".to_string()));
            eprintln!("[project] create_project failed: {:?}", app.last_error);
        }
    } else {
        eprintln!("[project] no folder selected");
    }
}

fn open_project_dialog(app: &mut RustToolsApp) {
    if let Some(path) = pick_folder_fallback() {
        eprintln!("[project] opening project at {}", path.display());
        let result = open_project(path.to_string_lossy().to_string());
        if result.success {
            if let Some(config) = result.data {
                eprintln!("[project] opened project '{}' with {} classes", config.name, config.classes.len());
                app.current_project = Some(config);
            }
        } else {
            let err = result.error.unwrap_or_else(|| "打开项目失败".to_string());
            eprintln!("[project] open_project failed: {}", err);
            app.last_error = Some(err);
        }
    } else {
        eprintln!("[project] no folder selected");
    }
}
