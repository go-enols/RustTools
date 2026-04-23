use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::app::RustToolsApp;
use crate::theme::{AppleColors, compact_card_frame};

/// 单个标注矩形
#[derive(Clone, Debug)]
pub struct AnnotationRect {
    pub class_id: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// 标注工具类型
#[derive(Default, PartialEq)]
pub enum AnnotationTool {
    #[default]
    Select,
    Pan,
    Rectangle,
}

/// 标注页面状态
pub struct AnnotationState {
    pub image_folder: Option<PathBuf>,
    pub images: Vec<PathBuf>,
    pub selected_image_idx: Option<usize>,
    pub annotations: HashMap<String, Vec<AnnotationRect>>,
    pub current_class: usize,
    pub zoom: f32,
    pub show_labels: bool,
    pub show_grid: bool,
    pub tool: AnnotationTool,
    pub drawing_start: Option<egui::Pos2>,
    pub selected_annotation: Option<usize>,
    /// 图片平移偏移（抓手工具拖动产生）
    pub img_offset: egui::Vec2,
    pan_start: Option<egui::Pos2>,
    textures: HashMap<String, egui::TextureHandle>,
}

impl Default for AnnotationState {
    fn default() -> Self {
        Self {
            image_folder: None,
            images: Vec::new(),
            selected_image_idx: None,
            annotations: HashMap::new(),
            current_class: 0,
            zoom: 1.0,
            show_labels: true,
            show_grid: false,
            tool: AnnotationTool::Select,
            drawing_start: None,
            selected_annotation: None,
            img_offset: egui::Vec2::ZERO,
            pan_start: None,
            textures: HashMap::new(),
        }
    }
}

impl AnnotationState {
    fn selected_image_name(&self) -> Option<String> {
        self.selected_image_idx
            .and_then(|i| self.images.get(i))
            .map(|p| p.file_stem().unwrap_or_default().to_string_lossy().to_string())
    }

    fn load_annotations_for_image(&mut self, image_path: &Path, project: Option<&crate::models::ProjectConfig>) {
        let stem = image_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        
        let label_path = if let Some(proj) = project {
            // 使用项目配置的 labels 路径
            let base = Path::new(&proj.path);
            // 判断图像是 train 还是 val
            let img_str = image_path.to_string_lossy();
            let subdir = if img_str.contains(&proj.images.train) {
                &proj.labels.train
            } else if img_str.contains(&proj.images.val) {
                &proj.labels.val
            } else {
                &proj.labels.train
            };
            Some(base.join(subdir).join(format!("{}.txt", stem)))
        } else {
            // 无项目时：尝试标准 YOLO 目录结构推断
            Self::infer_label_path(image_path)
        };
        
        if let Some(lp) = label_path {
            if lp.exists() {
                if let Ok(content) = std::fs::read_to_string(&lp) {
                    let mut rects = Vec::new();
                    for line in content.lines() {
                        let parts: Vec<f32> = line.split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if parts.len() >= 5 {
                            rects.push(AnnotationRect {
                                class_id: parts[0] as usize,
                                x: parts[1],
                                y: parts[2],
                                w: parts[3],
                                h: parts[4],
                            });
                        }
                    }
                    self.annotations.insert(stem, rects);
                }
            }
        }
    }

    fn save_annotations_for_image(&self, image_path: &Path, project: Option<&crate::models::ProjectConfig>) {
        let stem = image_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        if let Some(rects) = self.annotations.get(&stem) {
            let label_path = if let Some(proj) = project {
                let base = Path::new(&proj.path);
                let img_str = image_path.to_string_lossy();
                let subdir = if img_str.contains(&proj.images.train) {
                    &proj.labels.train
                } else if img_str.contains(&proj.images.val) {
                    &proj.labels.val
                } else {
                    &proj.labels.train
                };
                Some(base.join(subdir).join(format!("{}.txt", stem)))
            } else {
                Self::infer_label_path(image_path)
            };
            
            if let Some(lp) = label_path {
                if let Some(parent) = lp.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let content: String = rects.iter()
                    .map(|r| format!("{} {} {} {} {}", r.class_id, r.x, r.y, r.w, r.h))
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = std::fs::write(&lp, content);
            }
        }
    }

    /// 从图像路径推断标注路径（标准 YOLO 结构）
    fn infer_label_path(image_path: &Path) -> Option<PathBuf> {
        let stem = image_path.file_stem()?.to_string_lossy();
        let parent = image_path.parent()?;
        
        // 向上查找 images 目录
        let mut current = parent;
        loop {
            if let Some(dir_name) = current.file_name() {
                if dir_name.to_string_lossy() == "images" {
                    let project_root = current.parent()?;
                    let rel_from_images = parent.strip_prefix(current).ok()?;
                    return Some(project_root.join("labels").join(rel_from_images).join(format!("{}.txt", stem)));
                }
            }
            current = current.parent()?;
        }
    }

    fn get_or_load_texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();
        if let Some(tex) = self.textures.get(&key) {
            return Some(tex.clone());
        }
        
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
        let tex = ctx.load_texture(&key, color_image, egui::TextureOptions::LINEAR);
        self.textures.insert(key, tex.clone());
        Some(tex)
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let tc = app.colors();
    let state = &mut app.annotation_state;

    // 自动加载项目图片
    if let Some(ref project) = app.current_project {
        if state.images.is_empty() && state.image_folder.is_none() {
            let train_dir = std::path::Path::new(&project.path).join(&project.images.train);
            let val_dir = std::path::Path::new(&project.path).join(&project.images.val);
            if train_dir.exists() {
                load_images_from_folder(state, &train_dir);
            } else if val_dir.exists() {
                load_images_from_folder(state, &val_dir);
            }
        }
    }

    // 检查是否有项目或图像
    if app.current_project.is_none() && state.images.is_empty() {
        show_empty_state(ui, state, &tc);
        return;
    }

    // ── 全局快捷键 ──
    ui.ctx().input(|i| {
        // Q: 上一张
        if i.key_pressed(egui::Key::Q) {
            if let Some(idx) = state.selected_image_idx {
                if idx > 0 {
                    state.selected_image_idx = Some(idx - 1);
                    state.selected_annotation = None;
                    state.img_offset = egui::Vec2::ZERO;
                    let path = state.images[idx - 1].clone();
                    state.load_annotations_for_image(&path, app.current_project.as_ref());
                }
            }
        }
        // E: 下一张
        if i.key_pressed(egui::Key::E) {
            if let Some(idx) = state.selected_image_idx {
                if idx + 1 < state.images.len() {
                    state.selected_image_idx = Some(idx + 1);
                    state.selected_annotation = None;
                    state.img_offset = egui::Vec2::ZERO;
                    let path = state.images[idx + 1].clone();
                    state.load_annotations_for_image(&path, app.current_project.as_ref());
                }
            }
        }
        // W: 抓手工具
        if i.key_pressed(egui::Key::W) {
            state.tool = AnnotationTool::Pan;
        }
        // D: 矩形标注工具
        if i.key_pressed(egui::Key::D) {
            state.tool = AnnotationTool::Rectangle;
        }
        // R: 撤销（删除最后一个或选中的标注）
        if i.key_pressed(egui::Key::R) {
            if let Some(ref name) = state.selected_image_name() {
                if let Some(annots) = state.annotations.get_mut(name) {
                    if let Some(aidx) = state.selected_annotation {
                        if aidx < annots.len() {
                            annots.remove(aidx);
                        }
                        state.selected_annotation = None;
                    } else if !annots.is_empty() {
                        annots.pop();
                    }
                }
            }
        }
        // Delete: 删除选中标注
        if i.key_pressed(egui::Key::Delete) {
            if let (Some(aidx), Some(ref name)) = (state.selected_annotation, state.selected_image_name()) {
                if let Some(annots) = state.annotations.get_mut(name) {
                    if aidx < annots.len() {
                        annots.remove(aidx);
                        state.selected_annotation = None;
                    }
                }
            }
        }
        // 数字键 0-9 切换类别
        for (n, key) in [egui::Key::Num0, egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                         egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7,
                         egui::Key::Num8, egui::Key::Num9].iter().enumerate() {
            if i.key_pressed(*key) {
                let max_class = app.current_project.as_ref()
                    .map(|p| p.classes.len())
                    .unwrap_or(1);
                if n < max_class {
                    state.current_class = n;
                }
            }
        }
    });

    // 计算标注进度
    let total_images = state.images.len();
    let annotated_count = state.images.iter().filter(|p| {
        let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        state.annotations.get(&stem).map(|v| !v.is_empty()).unwrap_or(false)
    }).count();

    // ── 页面标题 + 进度条 ──
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("图像标注")
                    .size(20.0)
                    .strong()
                    .color(tc.text),
            );
            ui.label(
                egui::RichText::new("可视化图像标注工具")
                    .size(12.0)
                    .color(tc.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ui.add_sized([80.0, 32.0], egui::Button::new(
                egui::RichText::new("保存标注").color(tc.surface).strong()
            ).fill(AppleColors::PRIMARY).corner_radius(egui::CornerRadius::same(6))).clicked() {
                if let Some(ref path) = state.images.get(state.selected_image_idx.unwrap_or(0)) {
                    state.save_annotations_for_image(path, app.current_project.as_ref());
                }
            }
            if ui.add_sized([80.0, 32.0], egui::Button::new("打开文件夹").corner_radius(egui::CornerRadius::same(6))).clicked() {
                open_folder(state);
            }
        });
    });

    // 进度统计条
    if total_images > 0 {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let bar_w = ui.available_width().min(400.0);
            let bar_h = 6.0;
            let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h + 16.0), egui::Sense::hover());
            let painter = ui.painter();
            // 背景
            let bg_rect = egui::Rect::from_min_size(bar_rect.min + egui::vec2(0.0, 14.0), egui::vec2(bar_w, bar_h));
            painter.rect_filled(bg_rect, egui::CornerRadius::same(3), tc.bg_deep);
            // 进度
            let progress = annotated_count as f32 / total_images as f32;
            let fill_w = bar_w * progress;
            if fill_w > 0.0 {
                let fill_rect = egui::Rect::from_min_size(bg_rect.min, egui::vec2(fill_w, bar_h));
                painter.rect_filled(fill_rect, egui::CornerRadius::same(3), AppleColors::SUCCESS);
            }
            // 文字
            painter.text(
                bar_rect.min,
                egui::Align2::LEFT_TOP,
                &format!("已标注 {} / {} 张图像", annotated_count, total_images),
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
                tc.text_secondary,
            );
            let pct = (progress * 100.0) as u32;
            painter.text(
                bar_rect.max,
                egui::Align2::RIGHT_TOP,
                &format!("{}%", pct),
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
                AppleColors::SUCCESS,
            );
        });
    }
    ui.add_space(12.0);

    let available = ui.available_size();
    let left_w = 200.0_f32.min(available.x * 0.18).max(160.0);
    let right_w = 220.0_f32.min(available.x * 0.20).max(180.0);
    let center_w = available.x - left_w - right_w - 24.0;

    ui.horizontal_top(|ui| {
        // ── 左侧面板：图像列表 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("图像列表")
                                .size(13.0)
                                .strong()
                                .color(tc.text),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", state.images.len()))
                                .size(11.0)
                                .color(tc.text_secondary),
                        );
                    });
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .id_salt("image_list_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if state.images.is_empty() {
                                ui.label(
                                    egui::RichText::new("请先打开图像文件夹")
                                        .size(12.0)
                                        .color(tc.text_secondary),
                                );
                            } else {
                                let mut load_idx = None;
                                for (i, path) in state.images.iter().enumerate() {
                                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                    let is_selected = state.selected_image_idx == Some(i);
                                    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                    let has_annot = state.annotations.get(&stem).map(|v| !v.is_empty()).unwrap_or(false);

                                    let item_h = 36.0;
                                    let item_w = ui.available_width();
                                    let (item_rect, item_response) = ui.allocate_exact_size(
                                        egui::vec2(item_w, item_h),
                                        egui::Sense::click(),
                                    );
                                    let painter = ui.painter();

                                    // 背景
                                    if is_selected {
                                        painter.rect_filled(item_rect, egui::CornerRadius::same(6), AppleColors::PRIMARY.gamma_multiply(0.08));
                                        painter.rect_stroke(item_rect, egui::CornerRadius::same(6), egui::Stroke::new(1.0, AppleColors::PRIMARY.gamma_multiply(0.2)), egui::StrokeKind::Inside);
                                    } else if item_response.hovered() {
                                        painter.rect_filled(item_rect, egui::CornerRadius::same(6), tc.text.gamma_multiply(0.03));
                                    }

                                    // 缩略图占位框
                                    let thumb_rect = egui::Rect::from_min_size(
                                        item_rect.min + egui::vec2(6.0, 4.0),
                                        egui::vec2(28.0, 28.0),
                                    );
                                    painter.rect_filled(thumb_rect, egui::CornerRadius::same(4), tc.bg_deep);
                                    // 小型图片图标
                                    let img_icon = thumb_rect.shrink(6.0);
                                    let stroke = egui::Stroke::new(1.0, tc.text_tertiary);
                                    painter.rect_stroke(img_icon, egui::CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
                                    let m1 = img_icon.min + egui::vec2(img_icon.width() * 0.2, img_icon.height() * 0.7);
                                    let m2 = img_icon.min + egui::vec2(img_icon.width() * 0.5, img_icon.height() * 0.3);
                                    let m3 = img_icon.min + egui::vec2(img_icon.width() * 0.8, img_icon.height() * 0.7);
                                    painter.line_segment([m1, m2], stroke);
                                    painter.line_segment([m2, m3], stroke);

                                    // 文件名
                                    let text_color = if is_selected { AppleColors::PRIMARY } else { tc.text };
                                    let text_pos = item_rect.min + egui::vec2(40.0, item_rect.height() * 0.5);
                                    // 截断文件名
                                    let display_name = if name.len() > 18 { format!("{}..", &name[..16]) } else { name };
                                    painter.text(
                                        text_pos,
                                        egui::Align2::LEFT_CENTER,
                                        &display_name,
                                        egui::FontId::new(11.0, egui::FontFamily::Proportional),
                                        text_color,
                                    );

                                    // 标注状态圆点（右侧）
                                    let dot_center = item_rect.max - egui::vec2(12.0, item_rect.height() * 0.5);
                                    if has_annot {
                                        painter.circle_filled(dot_center, 4.0, AppleColors::SUCCESS);
                                    } else {
                                        painter.circle_stroke(dot_center, 3.5, egui::Stroke::new(1.0, tc.text_tertiary.gamma_multiply(0.5)));
                                    }

                                    if item_response.clicked() {
                                        state.selected_image_idx = Some(i);
                                        state.selected_annotation = None;
                                        load_idx = Some(i);
                                    }
                                }
                                if let Some(i) = load_idx {
                                    let path = state.images[i].clone();
                                    state.load_annotations_for_image(&path, app.current_project.as_ref());
                                }
                            }
                        });
                });
            },
        );

        ui.add_space(8.0);

        // ── 中央面板：画布区域 ──
        ui.allocate_ui_with_layout(
            egui::vec2(center_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    // 工具栏 - 图标风格
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        // 选择工具按钮
                        let select_active = state.tool == AnnotationTool::Select;
                        let sel_color = if select_active { AppleColors::PRIMARY } else { tc.text_secondary };
                        let sel_bg = if select_active { AppleColors::PRIMARY.gamma_multiply(0.1) } else { egui::Color32::TRANSPARENT };
                        let sel_response = ui.add_sized([32.0, 28.0], egui::Button::new("")
                            .fill(sel_bg)
                            .corner_radius(egui::CornerRadius::same(4))
                            .stroke(egui::Stroke::new(if select_active { 1.0 } else { 0.0 }, AppleColors::PRIMARY.gamma_multiply(0.3)))
                        );
                        // 绘制鼠标指针图标
                        let c = sel_response.rect.center();
                        let arrow_pts = [
                            c + egui::vec2(-4.0, -6.0),
                            c + egui::vec2(-4.0, 4.0),
                            c + egui::vec2(-1.0, 1.0),
                            c + egui::vec2(2.0, 6.0),
                            c + egui::vec2(4.0, 5.0),
                            c + egui::vec2(1.0, 0.0),
                            c + egui::vec2(5.0, 0.0),
                        ];
                        for i in 0..arrow_pts.len() {
                            ui.painter().line_segment([arrow_pts[i], arrow_pts[(i + 1) % arrow_pts.len()]], egui::Stroke::new(1.5, sel_color));
                        }
                        if sel_response.clicked() { state.tool = AnnotationTool::Select; }

                        // 抓手/平移工具按钮
                        let pan_active = state.tool == AnnotationTool::Pan;
                        let pan_color = if pan_active { AppleColors::PRIMARY } else { tc.text_secondary };
                        let pan_bg = if pan_active { AppleColors::PRIMARY.gamma_multiply(0.1) } else { egui::Color32::TRANSPARENT };
                        let pan_response = ui.add_sized([32.0, 28.0], egui::Button::new("")
                            .fill(pan_bg)
                            .corner_radius(egui::CornerRadius::same(4))
                            .stroke(egui::Stroke::new(if pan_active { 1.0 } else { 0.0 }, AppleColors::PRIMARY.gamma_multiply(0.3)))
                        );
                        let pc = pan_response.rect.center();
                        // 绘制抓手图标（手掌轮廓）
                        ui.painter().circle_filled(pc + egui::vec2(-2.0, 2.0), 2.5, pan_color);
                        ui.painter().circle_filled(pc + egui::vec2(2.0, 2.0), 2.5, pan_color);
                        ui.painter().circle_filled(pc + egui::vec2(-4.0, -2.0), 2.0, pan_color);
                        ui.painter().circle_filled(pc + egui::vec2(0.0, -2.0), 2.0, pan_color);
                        ui.painter().circle_filled(pc + egui::vec2(4.0, -2.0), 2.0, pan_color);
                        if pan_response.clicked() { state.tool = AnnotationTool::Pan; }

                        // 矩形工具按钮
                        let rect_active = state.tool == AnnotationTool::Rectangle;
                        let rect_color = if rect_active { AppleColors::PRIMARY } else { tc.text_secondary };
                        let rect_bg = if rect_active { AppleColors::PRIMARY.gamma_multiply(0.1) } else { egui::Color32::TRANSPARENT };
                        let rect_response = ui.add_sized([32.0, 28.0], egui::Button::new("")
                            .fill(rect_bg)
                            .corner_radius(egui::CornerRadius::same(4))
                            .stroke(egui::Stroke::new(if rect_active { 1.0 } else { 0.0 }, AppleColors::PRIMARY.gamma_multiply(0.3)))
                        );
                        let rc = rect_response.rect.center();
                        let r = egui::Rect::from_center_size(rc, egui::vec2(10.0, 8.0));
                        ui.painter().rect_stroke(r, egui::CornerRadius::same(2), egui::Stroke::new(1.5, rect_color), egui::StrokeKind::Inside);
                        if rect_response.clicked() { state.tool = AnnotationTool::Rectangle; }

                        ui.separator();

                        // 缩放控制
                        let zoom_btn = |ui: &mut egui::Ui, label: &str| -> egui::Response {
                            ui.add_sized([24.0, 24.0], egui::Button::new(
                                egui::RichText::new(label).size(12.0).strong()
                            ).corner_radius(egui::CornerRadius::same(4)))
                        };
                        if zoom_btn(ui, "-").clicked() {
                            state.zoom = (state.zoom - 0.1).max(0.1);
                        }
                        ui.label(egui::RichText::new(format!("{:.0}%", state.zoom * 100.0)).size(12.0).monospace());
                        if zoom_btn(ui, "+").clicked() {
                            state.zoom = (state.zoom + 0.1).min(5.0);
                        }

                        ui.separator();

                        // 标签/网格开关
                        let mut label_toggle = state.show_labels;
                        if ui.checkbox(&mut label_toggle, "标签").changed() {
                            state.show_labels = label_toggle;
                        }
                        let mut grid_toggle = state.show_grid;
                        if ui.checkbox(&mut grid_toggle, "网格").changed() {
                            state.show_grid = grid_toggle;
                        }
                    });
                    ui.add_space(4.0);

                    // 画布
                    let canvas_h = ui.available_height() - 4.0;
                    let canvas_size = egui::vec2(center_w - 28.0, canvas_h.max(100.0));
                    let canvas_id = ui.id().with("annotation_canvas");
                    let canvas_rect = ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag()).1.rect;
                    let ctx = ui.ctx().clone();

                    ui.painter().rect_filled(
                        canvas_rect,
                        egui::CornerRadius::same(8),
                        tc.bg_deep,
                    );

                    // 滚轮缩放
                    ui.ctx().input(|i| {
                        let scroll = i.smooth_scroll_delta.y;
                        if scroll != 0.0 && canvas_rect.contains(i.pointer.interact_pos().unwrap_or_default()) {
                            let factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                            state.zoom = (state.zoom * factor).clamp(0.1, 5.0);
                        }
                    });

                    let mut img_size = egui::Vec2::ZERO;
                    #[allow(unused_assignments)]
                    let mut img_rect = egui::Rect::ZERO;
                    if let Some(idx) = state.selected_image_idx {
                        let path = state.images.get(idx).cloned();
                        if let Some(ref path) = path {
                            if let Some(tex) = state.get_or_load_texture(&ctx, path) {
                                let tex_size = tex.size_vec2();
                                let canvas_size = canvas_rect.size();
                                let fit_scale = (canvas_size.x / tex_size.x)
                                    .min(canvas_size.y / tex_size.y)
                                    .min(1.0);
                                let base_size = tex_size * fit_scale;
                                let max_zoom = (canvas_size.x / base_size.x)
                                    .min(canvas_size.y / base_size.y);
                                let effective_zoom = state.zoom.clamp(0.1, max_zoom.max(1.0));
                                img_size = base_size * effective_zoom;
                                // 以画布中心为基准，加上平移偏移
                                let center = canvas_rect.center() + state.img_offset;
                                img_rect = egui::Rect::from_center_size(center, img_size);

                                // 裁剪到画布内（防止图片拖出太远）
                                let clamped_rect = img_rect.intersect(canvas_rect.expand(img_size.max_elem()));
                                if clamped_rect.is_positive() {
                                    img_rect = clamped_rect;
                                    // 重新计算偏移
                                    state.img_offset = img_rect.center() - canvas_rect.center();
                                }

                                ui.painter().image(
                                    tex.id(),
                                    img_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );

                                // 绘制已有标注（仅在大图预览中）
                                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                if let Some(annots) = state.annotations.get(&stem) {
                                    for (i, rect) in annots.iter().enumerate() {
                                        let rx = img_rect.min.x + rect.x * img_size.x;
                                        let ry = img_rect.min.y + rect.y * img_size.y;
                                        let rw = rect.w * img_size.x;
                                        let rh = rect.h * img_size.y;
                                        let r = egui::Rect::from_min_size(
                                            egui::pos2(rx - rw / 2.0, ry - rh / 2.0),
                                            egui::vec2(rw, rh),
                                        );

                                        let color = class_color(rect.class_id);
                                        let is_selected = state.selected_annotation == Some(i);
                                        let stroke_width = if is_selected { 3.0 } else { 2.0 };
                                        let painter = ui.painter();

                                        if is_selected {
                                            // 选中框：底层实线 + 蚂蚁线虚线流动动画 + 四角标记
                                            let time = ui.ctx().input(|i| i.time) as f32;
                                            let dash = 6.0;
                                            let gap = 4.0;
                                            let speed = 25.0;
                                            let offset = (time * speed) % (dash + gap);

                                            // 底层略宽实线
                                            painter.rect_stroke(
                                                r,
                                                egui::CornerRadius::same(2),
                                                egui::Stroke::new(stroke_width, color.gamma_multiply(0.6)),
                                                egui::StrokeKind::Inside,
                                            );

                                            // 蚂蚁线白色虚线（沿周长连续流动）
                                            let white = egui::Color32::WHITE;
                                            let top_w = r.width();
                                            let right_h = r.height();
                                            let bottom_w = r.width();

                                            draw_dashed_line(painter, r.left_top(), r.right_top(), dash, gap, offset, white);
                                            let off_right = (top_w + offset) % (dash + gap);
                                            draw_dashed_line(painter, r.right_top(), r.right_bottom(), dash, gap, off_right, white);
                                            let off_bottom = (top_w + right_h + offset) % (dash + gap);
                                            draw_dashed_line(painter, r.right_bottom(), r.left_bottom(), dash, gap, off_bottom, white);
                                            let off_left = (top_w + right_h + bottom_w + offset) % (dash + gap);
                                            draw_dashed_line(painter, r.left_bottom(), r.left_top(), dash, gap, off_left, white);

                                            // 四角调整标记
                                            let handle_len = 8.0;
                                            let handle_stroke = egui::Stroke::new(2.0, white);
                                            // 左上
                                            painter.line_segment([r.left_top(), r.left_top() + egui::vec2(handle_len, 0.0)], handle_stroke);
                                            painter.line_segment([r.left_top(), r.left_top() + egui::vec2(0.0, handle_len)], handle_stroke);
                                            // 右上
                                            painter.line_segment([r.right_top(), r.right_top() + egui::vec2(-handle_len, 0.0)], handle_stroke);
                                            painter.line_segment([r.right_top(), r.right_top() + egui::vec2(0.0, handle_len)], handle_stroke);
                                            // 右下
                                            painter.line_segment([r.right_bottom(), r.right_bottom() + egui::vec2(-handle_len, 0.0)], handle_stroke);
                                            painter.line_segment([r.right_bottom(), r.right_bottom() + egui::vec2(0.0, -handle_len)], handle_stroke);
                                            // 左下
                                            painter.line_segment([r.left_bottom(), r.left_bottom() + egui::vec2(handle_len, 0.0)], handle_stroke);
                                            painter.line_segment([r.left_bottom(), r.left_bottom() + egui::vec2(0.0, -handle_len)], handle_stroke);
                                        } else {
                                            painter.rect_stroke(
                                                r,
                                                egui::CornerRadius::same(2),
                                                egui::Stroke::new(stroke_width, color),
                                                egui::StrokeKind::Inside,
                                            );
                                        }

                                        if state.show_labels {
                                            let class_name = app.current_project.as_ref()
                                                .and_then(|p| p.classes.get(rect.class_id))
                                                .cloned()
                                                .unwrap_or_else(|| format!("class_{}", rect.class_id));
                                            ui.painter().text(
                                                r.min + egui::vec2(2.0, 12.0),
                                                egui::Align2::LEFT_TOP,
                                                &class_name,
                                                egui::FontId::new(11.0, egui::FontFamily::Proportional),
                                                color,
                                            );
                                        }
                                    }
                                }

                                // ── 抓手/平移工具 ──
                                if state.tool == AnnotationTool::Pan {
                                    let drag = ui.ctx().input(|i| i.pointer.delta());
                                    if ui.ctx().input(|i| i.pointer.primary_down()) {
                                        if canvas_rect.contains(ui.ctx().input(|i| i.pointer.interact_pos().unwrap_or_default())) {
                                            state.img_offset += drag;
                                        }
                                    }
                                }

                                // ── 矩形标注工具 ──
                                if state.tool == AnnotationTool::Rectangle {
                                    // 使用 canvas_rect 的响应来检测拖动
                                    let canvas_response = ui.interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());
                                    if canvas_response.drag_started() {
                                        if let Some(pos) = canvas_response.interact_pointer_pos() {
                                            if img_rect.contains(pos) {
                                                state.drawing_start = Some(pos);
                                            }
                                        }
                                    }

                                    if let Some(start) = state.drawing_start {
                                        if let Some(current) = canvas_response.interact_pointer_pos() {
                                            let r = egui::Rect::from_two_pos(start, current);
                                            ui.painter().rect_stroke(
                                                r,
                                                egui::CornerRadius::same(2),
                                                egui::Stroke::new(2.0, class_color(state.current_class)),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                    }

                                    if canvas_response.drag_stopped() {
                                        if let Some(start) = state.drawing_start {
                                            if let Some(end) = canvas_response.interact_pointer_pos() {
                                                if img_rect.contains(start) && img_rect.contains(end) {
                                                    let x1 = (start.x - img_rect.min.x) / img_size.x;
                                                    let y1 = (start.y - img_rect.min.y) / img_size.y;
                                                    let x2 = (end.x - img_rect.min.x) / img_size.x;
                                                    let y2 = (end.y - img_rect.min.y) / img_size.y;

                                                    let nx = (x1 + x2) / 2.0;
                                                    let ny = (y1 + y2) / 2.0;
                                                    let nw = (x2 - x1).abs();
                                                    let nh = (y2 - y1).abs();

                                                    if nw > 0.01 && nh > 0.01 {
                                                        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                                        let annots = state.annotations.entry(stem).or_default();
                                                        annots.push(AnnotationRect {
                                                            class_id: state.current_class,
                                                            x: nx.clamp(0.0, 1.0),
                                                            y: ny.clamp(0.0, 1.0),
                                                            w: nw.clamp(0.0, 1.0),
                                                            h: nh.clamp(0.0, 1.0),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        state.drawing_start = None;
                                    }
                                }

                                // ── 选择工具：点击选中已有标注 ──
                                if state.tool == AnnotationTool::Select {
                                    let canvas_response = ui.interact(canvas_rect, canvas_id, egui::Sense::click());
                                    if canvas_response.clicked() {
                                        if let Some(pos) = canvas_response.interact_pointer_pos() {
                                            let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                            if let Some(annots) = state.annotations.get(&stem) {
                                                let mut found = None;
                                                for (i, rect) in annots.iter().enumerate().rev() {
                                                    let rx = img_rect.min.x + rect.x * img_size.x;
                                                    let ry = img_rect.min.y + rect.y * img_size.y;
                                                    let rw = rect.w * img_size.x;
                                                    let rh = rect.h * img_size.y;
                                                    let r = egui::Rect::from_min_size(
                                                        egui::pos2(rx - rw / 2.0, ry - rh / 2.0),
                                                        egui::vec2(rw, rh),
                                                    );
                                                    if r.contains(pos) {
                                                        found = Some(i);
                                                        break;
                                                    }
                                                }
                                                state.selected_annotation = found;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if img_size == egui::Vec2::ZERO {
                        let center = canvas_rect.center();
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "选择图像以开始标注",
                            egui::FontId::new(14.0, egui::FontFamily::Proportional),
                            tc.text_secondary,
                        );
                    }
                });
            },
        );

        ui.add_space(8.0);

        // ── 右侧面板：类别与属性 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("类别列表")
                            .size(13.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(8.0);

                    let classes: Vec<String> = app.current_project.as_ref()
                        .map(|p| p.classes.clone())
                        .unwrap_or_else(|| vec!["object".to_string()]);

                    for (i, class) in classes.iter().enumerate() {
                        let is_selected = state.current_class == i;
                        let color = class_color(i);

                        let item_response = ui.selectable_label(is_selected, "");
                        let item_rect = item_response.rect;
                        let painter = ui.painter();

                        // 彩色圆点标记
                        let dot_center = item_rect.min + egui::vec2(10.0, item_rect.height() * 0.5);
                        painter.circle_filled(dot_center, 5.0, color);

                        // 类别文字
                        let text_color = if is_selected { tc.text } else { tc.text_secondary };
                        let text_str = format!("{}: {}", i, class);
                        let text_pos = item_rect.min + egui::vec2(24.0, item_rect.height() * 0.5);
                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            &text_str,
                            egui::FontId::new(12.0, egui::FontFamily::Proportional),
                            text_color,
                        );

                        if item_response.clicked() {
                            state.current_class = i;
                        }
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("当前标注")
                            .size(13.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(4.0);

                    let annot_count = state.selected_image_name()
                        .and_then(|n| state.annotations.get(&n))
                        .map(|v| v.len())
                        .unwrap_or(0);
                    ui.label(
                        egui::RichText::new(format!("{} 个标注", annot_count))
                            .size(11.0)
                            .color(tc.text_secondary),
                    );

                    if let Some(ref name) = state.selected_image_name() {
                        if let Some(annots) = state.annotations.get(name) {
                            egui::ScrollArea::vertical()
                                .id_salt("annot_list_scroll")
                                .max_height(120.0)
                                .show(ui, |ui| {
                                    for (i, rect) in annots.iter().enumerate() {
                                        let is_sel = state.selected_annotation == Some(i);
                                        let class_name = classes.get(rect.class_id)
                                            .cloned()
                                            .unwrap_or_else(|| format!("class_{}", rect.class_id));
                                        let text = format!(
                                            "[{}] {}: {:.3},{:.3},{:.3},{:.3}",
                                            i, class_name, rect.x, rect.y, rect.w, rect.h
                                        );
                                        if ui.selectable_label(is_sel, &text).clicked() {
                                            state.selected_annotation = Some(i);
                                        }
                                    }
                                });
                        }
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("快捷键")
                            .size(13.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(4.0);
                    shortcut(ui, "Q / E", "上/下一张", &tc);
                    shortcut(ui, "W", "抓手平移", &tc);
                    shortcut(ui, "D", "矩形标注", &tc);
                    shortcut(ui, "R", "撤销", &tc);
                    shortcut(ui, "数字键", "切换类别", &tc);
                    shortcut(ui, "滚轮", "缩放", &tc);
                    shortcut(ui, "Delete", "删除选中", &tc);
                    ui.add_space(8.0);

                    if ui.button("删除选中").clicked() {
                        if let (Some(aidx), Some(ref name)) = (state.selected_annotation, state.selected_image_name()) {
                            if let Some(annots) = state.annotations.get_mut(name) {
                                if aidx < annots.len() {
                                    annots.remove(aidx);
                                    state.selected_annotation = None;
                                }
                            }
                        }
                    }
                });
            },
        );
    });
}

fn show_empty_state(ui: &mut egui::Ui, state: &mut AnnotationState, tc: &crate::theme::ThemeColors) {
    let available = ui.available_size();
    ui.allocate_ui_with_layout(
        egui::vec2(available.x, available.y * 0.7),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(60.0);

            // 绘制柔和的图片轮廓图标
            let icon_size = 56.0;
            let icon_rect = ui.allocate_exact_size(egui::vec2(icon_size, icon_size), egui::Sense::hover()).1.rect;
            let painter = ui.painter();
            let stroke = egui::Stroke::new(2.0, tc.text_tertiary);
            let img_rect = icon_rect.shrink(6.0);
            // 主体
            painter.rect_stroke(img_rect, egui::CornerRadius::same(6), stroke, egui::StrokeKind::Inside);
            // 山形图案
            let m1 = img_rect.min + egui::vec2(img_rect.width() * 0.25, img_rect.height() * 0.65);
            let m2 = img_rect.min + egui::vec2(img_rect.width() * 0.5, img_rect.height() * 0.35);
            let m3 = img_rect.min + egui::vec2(img_rect.width() * 0.75, img_rect.height() * 0.65);
            painter.line_segment([m1, m2], stroke);
            painter.line_segment([m2, m3], stroke);
            // 太阳/圆点
            let sun_center = img_rect.min + egui::vec2(img_rect.width() * 0.7, img_rect.height() * 0.25);
            painter.circle_stroke(sun_center, 5.0, stroke);

            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("未打开项目或图像")
                    .size(18.0)
                    .strong()
                    .color(tc.text),
            );
            ui.label(
                egui::RichText::new("请先打开一个项目，或选择图像文件夹进行标注。")
                    .color(tc.text_secondary),
            );
            ui.add_space(16.0);
            if ui.button("打开图像文件夹").clicked() {
                open_folder(state);
            }
        },
    );
}

fn open_folder(state: &mut AnnotationState) {
    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
        load_images_from_folder(state, &folder);
    }
}

fn load_images_from_folder(state: &mut AnnotationState, folder: &std::path::Path) {
    state.image_folder = Some(folder.to_path_buf());
    state.images.clear();
    state.selected_image_idx = None;
    state.annotations.clear();
    state.textures.clear();

    let extensions: std::collections::HashSet<&str> =
        ["jpg", "jpeg", "png", "bmp", "webp"].iter().cloned().collect();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if extensions.contains(ext.to_string_lossy().to_lowercase().as_str()) {
                    state.images.push(path);
                }
            }
        }
        state.images.sort();
    }
}

fn shortcut(ui: &mut egui::Ui, key: &str, desc: &str, tc: &crate::theme::ThemeColors) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [50.0, 18.0],
            egui::Label::new(
                egui::RichText::new(key)
                    .size(10.0)
                    .monospace()
                    .color(AppleColors::PRIMARY),
            ),
        );
        ui.label(
            egui::RichText::new(desc)
                .size(11.0)
                .color(tc.text_secondary),
        );
    });
}

fn class_color(class_id: usize) -> egui::Color32 {
    let colors = [
        egui::Color32::from_rgb(255, 59, 48),
        egui::Color32::from_rgb(52, 199, 89),
        egui::Color32::from_rgb(0, 122, 255),
        egui::Color32::from_rgb(255, 149, 0),
        egui::Color32::from_rgb(175, 82, 222),
        egui::Color32::from_rgb(255, 55, 95),
        egui::Color32::from_rgb(90, 200, 250),
        egui::Color32::from_rgb(255, 204, 0),
    ];
    colors[class_id % colors.len()]
}

/// 绘制沿周长连续流动的虚线线段
fn draw_dashed_line(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    dash: f32,
    gap: f32,
    offset: f32,
    color: egui::Color32,
) {
    let dir = end - start;
    let len = dir.length();
    if len < 0.001 {
        return;
    }
    let dir_norm = dir / len;
    let cycle = dash + gap;
    let mut pos = -offset;
    while pos < len {
        let s = pos.max(0.0);
        let e = (pos + dash).min(len);
        if e > s {
            let p1 = start + dir_norm * s;
            let p2 = start + dir_norm * e;
            painter.line_segment([p1, p2], egui::Stroke::new(1.5, color));
        }
        pos += cycle;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_infer_label_path_train_subdir() {
        let img = PathBuf::from("/project/images/train/img_001.jpg");
        let label = AnnotationState::infer_label_path(&img);
        assert!(label.is_some());
        assert_eq!(label.unwrap(), PathBuf::from("/project/labels/train/img_001.txt"));
    }

    #[test]
    fn test_infer_label_path_val_subdir() {
        let img = PathBuf::from("/project/images/val/img_002.png");
        let label = AnnotationState::infer_label_path(&img);
        assert!(label.is_some());
        assert_eq!(label.unwrap(), PathBuf::from("/project/labels/val/img_002.txt"));
    }

    #[test]
    fn test_infer_label_path_no_subdir() {
        let img = PathBuf::from("/project/images/photo.jpg");
        let label = AnnotationState::infer_label_path(&img);
        assert!(label.is_some());
        assert_eq!(label.unwrap(), PathBuf::from("/project/labels/photo.txt"));
    }

    #[test]
    fn test_infer_label_path_no_images_dir() {
        let img = PathBuf::from("/random/path/photo.jpg");
        let label = AnnotationState::infer_label_path(&img);
        assert!(label.is_none());
    }
}
