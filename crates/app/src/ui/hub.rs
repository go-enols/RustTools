use eframe::egui;
use crate::app::{RustToolsApp, Route};
use crate::theme::{AppleColors, module_gradient, compact_card_frame};

/// Hub 引导页面 - 有项目时显示项目概览 Dashboard
pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let tc = app.colors();
    let available = ui.available_size();

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        // ── 品牌区 ──
        ui.add_space(available.y * 0.06);

        // 品牌图标
        let logo_size = 40.0;
        let (logo_rect, _) = ui.allocate_exact_size(egui::vec2(logo_size, logo_size), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(logo_rect, egui::CornerRadius::same(10), AppleColors::PRIMARY);
        let inner = logo_rect.shrink(logo_size * 0.3);
        painter.rect_filled(inner, egui::CornerRadius::same(3), tc.surface);
        ui.add_space(8.0);

        ui.label(
            egui::RichText::new("RustTools")
                .size(22.0)
                .strong()
                .color(tc.text),
        );
        ui.label(
            egui::RichText::new("多功能视觉开发工具箱")
                .size(12.0)
                .color(tc.text_secondary),
        );
        ui.add_space(4.0);

        // ── 环境状态概览卡片 ──
        ui.add_space(12.0);
        compact_card_frame().show(ui, |ui| {
            ui.set_max_width(available.x.min(480.0));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("运行环境").size(12.0).strong().color(tc.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("去设置 →").size(11.0).color(AppleColors::PRIMARY)).clicked() {
                        app.route = Route::Settings;
                    }
                });
            });
            ui.add_space(8.0);

            let status = &app.python_env_status;
            egui::Grid::new("hub_env_grid").num_columns(3).spacing([24.0, 6.0]).show(ui, |ui| {
                // Row: Python
                hub_env_item(ui, "Python", status.python_available, status.python_version.as_deref().unwrap_or("未安装"));
                ui.end_row();
                // Row: PyTorch
                let torch_text = if status.torch_available {
                    status.torch_version.as_deref().unwrap_or("已安装")
                } else { "未安装" };
                hub_env_item(ui, "PyTorch", status.torch_available, torch_text);
                ui.end_row();
                // Row: CUDA
                let cuda_text = if status.cuda_available { "可用" } else { "未检测" };
                hub_env_item(ui, "CUDA", status.cuda_available, cuda_text);
                ui.end_row();
            });

            // 环境未就绪提示
            if !status.python_available {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let warning_rect = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover()).1.rect;
                    ui.painter().text(
                        warning_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⚠",
                        egui::FontId::new(13.0, egui::FontFamily::Proportional),
                        AppleColors::WARNING,
                    );
                    ui.colored_label(
                        AppleColors::WARNING,
                        "Python 环境未就绪，部分功能（训练、推理）将不可用。请点击右上角「去设置」安装。"
                    );
                });
            }
        });

        // ── 项目概览（如有项目） ──
        if let Some(ref project) = app.current_project {
            ui.add_space(16.0);
            compact_card_frame().show(ui, |ui| {
                ui.set_max_width(available.x.min(640.0));
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&project.name)
                            .size(16.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.label(
                        egui::RichText::new(format!("YOLO {}", project.yolo_version))
                            .size(11.0)
                            .color(tc.text_secondary),
                    );
                });
                ui.add_space(8.0);

                let scan = scan_project_contents(project);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    hub_stat(ui, "训练图像", &format!("{}", scan.train_images), AppleColors::PRIMARY);
                    hub_stat(ui, "验证图像", &format!("{}", scan.val_images), AppleColors::WARNING);
                    hub_stat(ui, "标注数", &format!("{}", scan.total_annotations), AppleColors::SUCCESS);
                    hub_stat(ui, "类别", &format!("{}", project.classes.len()), AppleColors::PURPLE);
                    hub_stat(ui, "模型", &format!("{}", scan.model_count), AppleColors::TEAL);
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let annot_btn = egui::Button::new(
                        egui::RichText::new("图像标注").color(tc.surface).strong(),
                    )
                    .fill(AppleColors::PINK)
                    .corner_radius(egui::CornerRadius::same(6));
                    if ui.add_sized([90.0, 32.0], annot_btn).clicked() {
                        app.route = Route::Annotation;
                    }
                    let train_btn = egui::Button::new(
                        egui::RichText::new("开始训练").color(tc.surface).strong(),
                    )
                    .fill(AppleColors::SUCCESS)
                    .corner_radius(egui::CornerRadius::same(6));
                    if ui.add_sized([90.0, 32.0], train_btn).clicked() {
                        app.route = Route::Training;
                    }
                });
            });
        }

        ui.add_space(available.y * 0.06);

        // ── 模块卡片网格 ──
        let gap = 16.0;
        let cols = 3.0;
        let card_w = ((available.x - gap * (cols - 1.0)) / cols).min(260.0);
        let card_h = 140.0;

        let routes_row1 = [Route::Project, Route::Annotation, Route::Training];
        let routes_row2 = [Route::Video, Route::Desktop, Route::Device];

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space((available.x - card_w * cols - gap * (cols - 1.0)).max(0.0) * 0.5);
            for route in routes_row1 {
                module_entry_card(ui, card_w, card_h, route, app);
            }
        });

        ui.add_space(gap);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space((available.x - card_w * cols - gap * (cols - 1.0)).max(0.0) * 0.5);
            for route in routes_row2 {
                module_entry_card(ui, card_w, card_h, route, app);
            }
        });

        ui.add_space(available.y * 0.04);

        // ── 底部信息 ──
        ui.label(
            egui::RichText::new(format!("RustTools v{}", env!("CARGO_PKG_VERSION")))
                .size(11.0)
                .color(tc.text_tertiary),
        );
    });
}

fn hub_env_item(ui: &mut egui::Ui, label: &str, ok: bool, value: &str) {
    let color = if ok { AppleColors::SUCCESS } else { AppleColors::DANGER };
    ui.label(egui::RichText::new(label).size(11.0).color(AppleColors::TEXT_SECONDARY));
    let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(dot_rect.center(), 3.5, color);
    ui.label(egui::RichText::new(value).size(11.0).color(color));
}

fn hub_stat(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(value).size(16.0).strong().color(color));
        ui.label(egui::RichText::new(label).size(10.0).color(AppleColors::TEXT_SECONDARY));
    });
}

#[derive(Default)]
struct ProjectScanResult {
    train_images: usize,
    val_images: usize,
    total_annotations: usize,
    model_count: usize,
}

fn scan_project_contents(project: &crate::models::ProjectConfig) -> ProjectScanResult {
    let mut result = ProjectScanResult::default();
    let base = std::path::Path::new(&project.path);

    let train_img_dir = base.join(&project.images.train);
    if let Ok(entries) = std::fs::read_dir(&train_img_dir) {
        result.train_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

    let val_img_dir = base.join(&project.images.val);
    if let Ok(entries) = std::fs::read_dir(&val_img_dir) {
        result.val_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

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

    let models_dir = base.join("models");
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext.to_string_lossy().eq_ignore_ascii_case("pt") {
                    result.model_count += 1;
                }
            }
        }
    }

    result
}

fn module_entry_card(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    route: Route,
    app: &mut RustToolsApp,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();

    let (brand, _) = module_gradient(route);
    let label = route.to_string();

    // Shadow
    if hovered {
        painter.rect_filled(
            rect.expand(4.0),
            egui::CornerRadius::same(16),
            AppleColors::SHADOW_HOVER,
        );
    } else {
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, 2.0)),
            egui::CornerRadius::same(14),
            AppleColors::SHADOW,
        );
    }

    // Card background
    painter.rect_filled(rect, egui::CornerRadius::same(14), AppleColors::SURFACE);

    // Top accent bar
    let top_bar = egui::Rect::from_min_size(
        rect.min + egui::vec2(16.0, 0.0),
        egui::vec2(rect.width() - 32.0, 3.0),
    );
    painter.rect_filled(top_bar, egui::CornerRadius::same(2), brand);

    // Border
    let border_color = if hovered { brand } else { AppleColors::BORDER };
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(14),
        egui::Stroke::new(1.0, border_color),
        egui::StrokeKind::Inside,
    );

    // Content - vertically centered
    let content_rect = rect.shrink(16.0);
    let center_x = content_rect.center().x;

    // Icon background circle
    let icon_size = 44.0;
    let icon_y = content_rect.min.y + (content_rect.height() - 56.0) * 0.3 + 4.0;
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(center_x, icon_y + icon_size * 0.5),
        egui::vec2(icon_size, icon_size),
    );
    let icon_bg = if hovered { brand } else { brand.gamma_multiply(0.1) };
    painter.circle_filled(icon_rect.center(), icon_size * 0.5, icon_bg);

    // Draw geometric icon
    let icon_color = if hovered { AppleColors::SURFACE } else { brand };
    crate::ui::icons::draw_nav_icon(painter, icon_rect.shrink(10.0), route, icon_color);

    // Title
    let title_y = icon_rect.max.y + 16.0;
    painter.text(
        egui::pos2(center_x, title_y),
        egui::Align2::CENTER_TOP,
        label,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
        AppleColors::TEXT,
    );

    // Description
    let desc = match route {
        Route::Project => "创建与管理 YOLO 项目",
        Route::Annotation => "图像标注与数据集制作",
        Route::Training => "模型训练与超参调优",
        Route::Video => "视频文件推理分析",
        Route::Desktop => "实时屏幕捕获检测",
        Route::Device => "GPU/CPU 设备信息",
        _ => "",
    };
    painter.text(
        egui::pos2(center_x, title_y + 22.0),
        egui::Align2::CENTER_TOP,
        desc,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
        AppleColors::TEXT_SECONDARY,
    );

    if response.clicked() {
        app.route = route;
    }

    response
}
