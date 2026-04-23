use eframe::egui;
use crate::app::RustToolsApp;
use crate::theme::{AppleColors, compact_card_frame, page_header};
use crate::ui::desktop::DetectionResult;

#[derive(Debug, Clone)]
pub enum InferenceEvent {
    Progress { frame: u32, detections: usize },
    Frame { image_bytes: Vec<u8>, detections: Vec<DetectionResult>, frame: u32 },
    Complete { total_frames: u32, total_detections: usize },
    Error(String),
}

/// 视频推理页面状态
pub struct VideoPageState {
    pub video_path: Option<String>,
    pub model_idx: usize,
    pub confidence: f32,
    pub iou_threshold: f32,
    #[allow(dead_code)]
    pub save_results: bool,
    pub save_txt: bool,
    pub show_labels: bool,
    pub output_dir: Option<String>,
    pub is_processing: bool,
    pub progress: f32,
    pub current_frame: u32,
    pub total_frames: u32,
    pub detected_objects: usize,
    pub avg_confidence: f32,
    pub fps: f32,
    pub inference_rx: Option<std::sync::mpsc::Receiver<InferenceEvent>>,
    pub status_message: Option<String>,
    pub last_frame: Option<egui::TextureHandle>,
    pub last_detections: Vec<DetectionResult>,
    pub last_frame_idx: u32,
    pub img_width: u32,
    pub img_height: u32,
    pub last_model_idx: usize,
    pub last_confidence: f32,
    pub last_iou_threshold: f32,
    pub child_kill_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl Default for VideoPageState {
    fn default() -> Self {
        Self {
            video_path: None,
            model_idx: 0,
            confidence: 0.25,
            iou_threshold: 0.45,
            save_results: false,
            save_txt: false,
            show_labels: true,
            output_dir: None,
            is_processing: false,
            progress: 0.0,
            current_frame: 0,
            total_frames: 0,
            detected_objects: 0,
            avg_confidence: 0.0,
            fps: 0.0,
            inference_rx: None,
            status_message: None,
            last_frame: None,
            last_detections: Vec::new(),
            last_frame_idx: 0,
            img_width: 0,
            img_height: 0,
            last_model_idx: 0,
            last_confidence: 0.25,
            last_iou_threshold: 0.45,
            child_kill_tx: None,
        }
    }
}

const VIDEO_MODELS: &[&str] = &["yolo11n.pt", "yolo11s.pt", "yolo11m.pt", "自定义模型..."];

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let tc = app.colors();
    // 收集推理事件
    let events: Vec<InferenceEvent> = if let Some(ref rx) = app.video_state.inference_rx {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    } else {
        Vec::new()
    };

    let mut need_restart = false;
    for event in events {
        match event {
            InferenceEvent::Progress { frame, detections } => {
                app.video_state.current_frame = frame;
                app.video_state.detected_objects = detections;
                if app.video_state.total_frames > 0 {
                    app.video_state.progress = frame as f32 / app.video_state.total_frames as f32;
                }
            }
            InferenceEvent::Frame { image_bytes, detections, frame } => {
                if !image_bytes.is_empty() {
                    if let Ok(img) = image::load_from_memory(&image_bytes) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        let tex = ui.ctx().load_texture(
                            format!("video_frame_{}", frame),
                            color_image,
                            egui::TextureOptions::default(),
                        );
                        app.video_state.last_frame = Some(tex);
                        app.video_state.img_width = rgba.width();
                        app.video_state.img_height = rgba.height();
                    }
                }
                app.video_state.last_detections = detections;
                app.video_state.last_frame_idx = frame;
                app.video_state.current_frame = frame;
            }
            InferenceEvent::Complete { total_frames, total_detections } => {
                app.video_state.is_processing = false;
                app.video_state.inference_rx = None;
                app.video_state.child_kill_tx = None;
                app.video_state.current_frame = total_frames;
                app.video_state.detected_objects = total_detections;
                app.video_state.progress = 1.0;
                app.video_state.status_message = Some(format!(
                    "完成! 共处理 {} 帧, 检测到 {} 个目标",
                    total_frames, total_detections
                ));
            }
            InferenceEvent::Error(e) => {
                app.video_state.is_processing = false;
                app.video_state.inference_rx = None;
                app.video_state.child_kill_tx = None;
                app.video_state.status_message = Some(format!("错误: {}", e));
            }
        }
    }

    // 配置变化时自动重启推理
    if app.video_state.is_processing {
        let changed = app.video_state.model_idx != app.video_state.last_model_idx
            || (app.video_state.confidence - app.video_state.last_confidence).abs() > 0.001
            || (app.video_state.iou_threshold - app.video_state.last_iou_threshold).abs() > 0.001;
        if changed {
            need_restart = true;
            // 发送终止信号给 Python 子进程
            if let Some(ref tx) = app.video_state.child_kill_tx {
                let _ = tx.send(());
            }
            app.video_state.is_processing = false;
            app.video_state.inference_rx = None;
            app.video_state.child_kill_tx = None;
            app.video_state.status_message = Some("配置已更新，正在重新启动推理...".to_string());
        }
    }

    page_header(ui, "视频推理", "对视频文件进行目标检测推理");

    // 检查环境
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
            ui.label("Python 环境未就绪，推理功能不可用。");
        });
        ui.add_space(12.0);
    }

    let available = ui.available_size();
    let left_w = (available.x * 0.35).min(360.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：配置面板 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("推理配置")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(12.0);

                    // 视频文件选择
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("视频文件:").color(tc.text_secondary));
                        if ui.button("选择").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("视频文件", &["mp4", "avi", "mov", "mkv"])
                                .pick_file()
                            {
                                app.video_state.video_path = Some(path.to_string_lossy().to_string());
                            }
                        }
                    });
                    ui.add_space(2.0);
                    let path_text = app.video_state.video_path.as_deref().unwrap_or("未选择文件");
                    ui.label(
                        egui::RichText::new(path_text)
                            .size(11.0)
                            .monospace()
                            .color(tc.text_secondary),
                    );
                    ui.add_space(10.0);

                    // 模型选择
                    config_row(ui, "模型:", |ui| {
                        let model_name = VIDEO_MODELS.get(app.video_state.model_idx).unwrap_or(&VIDEO_MODELS[0]);
                        egui::ComboBox::from_id_salt("video_model")
                            .selected_text(*model_name)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in VIDEO_MODELS.iter().enumerate() {
                                    ui.selectable_value(&mut app.video_state.model_idx, i, *name);
                                }
                            });
                    }, &tc);
                    ui.add_space(6.0);

                    // 置信度阈值
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("置信度:").color(tc.text_secondary));
                        ui.add(egui::Slider::new(&mut app.video_state.confidence, 0.01..=1.0).show_value(true));
                    });
                    ui.add_space(4.0);

                    // IOU 阈值
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("IoU:").color(tc.text_secondary));
                        ui.add(egui::Slider::new(&mut app.video_state.iou_threshold, 0.1..=0.9).show_value(true));
                    });
                    ui.add_space(10.0);

                    ui.checkbox(&mut app.video_state.save_txt, "保存检测结果到文本");
                    ui.checkbox(&mut app.video_state.show_labels, "显示置信度标签");
                    ui.add_space(10.0);

                    // 输出路径
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("输出目录:").color(tc.text_secondary));
                        if ui.button("选择").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                app.video_state.output_dir = Some(path.to_string_lossy().to_string());
                            }
                        }
                    });
                    ui.add_space(2.0);
                    let out_text = app.video_state.output_dir.as_deref().unwrap_or("默认: 视频同级目录/output");
                    ui.label(
                        egui::RichText::new(out_text)
                            .size(11.0)
                            .color(tc.text_secondary),
                    );

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("计算设备").size(12.0).strong().color(tc.text),
                    );
                    ui.horizontal(|ui| {
                        ui.label("设备:");
                        let (device_text, dot_color) = if app.python_env_status.cuda_available {
                            ("GPU (CUDA)", AppleColors::SUCCESS)
                        } else {
                            ("CPU", tc.text_secondary)
                        };
                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 3.5, dot_color);
                        ui.label(device_text);
                    });

                    ui.add_space(12.0);

                    // 开始按钮
                    let can_start = app.python_env_status.python_available
                        && app.video_state.video_path.is_some()
                        && !app.video_state.is_processing;
                    let start_btn = egui::Button::new(
                        egui::RichText::new("开始推理").color(tc.surface).strong(),
                    )
                    .fill(AppleColors::PRIMARY)
                    .corner_radius(egui::CornerRadius::same(8));
                    if ui.add_sized([ui.available_width(), 40.0], start_btn).clicked() && can_start {
                        start_video_inference(app);
                    }
                });
            },
        );

        ui.add_space(8.0);

        // ── 右侧：预览与进度 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 视频预览区域
                compact_card_frame().show(ui, |ui| {
                    let _state = &mut app.video_state;
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("视频预览")
                                .size(14.0)
                                .strong()
                                .color(tc.text),
                        );
                        if let Some(ref msg) = app.video_state.status_message {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(msg)
                                        .size(11.0)
                                        .color(tc.text_secondary),
                                );
                            });
                        }
                    });
                    ui.add_space(8.0);

                    let preview_h = (ui.available_height() * 0.55).clamp(180.0, 320.0);
                    let preview_rect = ui.allocate_exact_size(
                        egui::vec2(right_w - 28.0, preview_h),
                        egui::Sense::hover(),
                    ).1.rect;

                    ui.painter().rect_filled(
                        preview_rect,
                        egui::CornerRadius::same(8),
                        tc.bg_deep,
                    );

                    // 显示最新帧
                    if let Some(ref tex) = app.video_state.last_frame {
                        let img_w = app.video_state.img_width as f32;
                        let img_h = app.video_state.img_height as f32;
                        if img_w > 0.0 && img_h > 0.0 {
                            let rect_aspect = preview_rect.width() / preview_rect.height().max(1.0);
                            let img_aspect = img_w / img_h;
                            let draw_rect = if img_aspect > rect_aspect {
                                let h = preview_rect.width() / img_aspect;
                                egui::Rect::from_center_size(
                                    preview_rect.center(),
                                    egui::vec2(preview_rect.width(), h),
                                )
                            } else {
                                let w = preview_rect.height() * img_aspect;
                                egui::Rect::from_center_size(
                                    preview_rect.center(),
                                    egui::vec2(w, preview_rect.height()),
                                )
                            };
                            ui.painter().image(
                                tex.id(),
                                draw_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );

                            // 绘制检测框
                            if app.video_state.show_labels && !app.video_state.last_detections.is_empty() {
                                let scale_x = draw_rect.width() / img_w;
                                let scale_y = draw_rect.height() / img_h;
                                for det in &app.video_state.last_detections {
                                    let r = egui::Rect::from_two_pos(
                                        draw_rect.min + egui::vec2(det.x1 * scale_x, det.y1 * scale_y),
                                        draw_rect.min + egui::vec2(det.x2 * scale_x, det.y2 * scale_y),
                                    );
                                    let color = class_color(det.class_id);
                                    ui.painter().rect_stroke(
                                        r,
                                        egui::CornerRadius::same(2),
                                        egui::Stroke::new(2.0, color),
                                        egui::StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        r.min + egui::vec2(2.0, 12.0),
                                        egui::Align2::LEFT_TOP,
                                        &format!("{:.0}%", det.confidence * 100.0),
                                        egui::FontId::new(11.0, egui::FontFamily::Proportional),
                                        color,
                                    );
                                }
                            }
                        }
                    } else if !app.video_state.is_processing {
                        let center = preview_rect.center();
                        let painter = ui.painter();
                        let play_size = 32.0;
                        let play_rect = egui::Rect::from_center_size(center - egui::vec2(0.0, 8.0), egui::vec2(play_size, play_size));
                        let stroke = egui::Stroke::new(1.5, tc.text_tertiary.gamma_multiply(0.5));
                        painter.circle_stroke(play_rect.center(), play_size * 0.5, stroke);
                        let tri_left = play_rect.center() + egui::vec2(-play_size * 0.15, -play_size * 0.2);
                        let tri_top = play_rect.center() + egui::vec2(play_size * 0.2, 0.0);
                        let tri_bottom = play_rect.center() + egui::vec2(-play_size * 0.15, play_size * 0.2);
                        painter.line_segment([tri_left, tri_top], stroke);
                        painter.line_segment([tri_top, tri_bottom], stroke);
                        painter.line_segment([tri_bottom, tri_left], stroke);

                        painter.text(
                            center + egui::vec2(0.0, 24.0),
                            egui::Align2::CENTER_CENTER,
                            "选择视频文件",
                            egui::FontId::new(13.0, egui::FontFamily::Proportional),
                            tc.text_secondary,
                        );
                    }
                });

                ui.add_space(8.0);

                // 进度与结果
                compact_card_frame().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("处理进度")
                            .size(14.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(8.0);

                    if app.video_state.is_processing {
                        ui.horizontal(|ui| {
                            ui.label(format!("帧: {}/{}", app.video_state.current_frame, app.video_state.total_frames));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{:.1}%", app.video_state.progress * 100.0));
                            });
                        });
                        ui.add(egui::ProgressBar::new(app.video_state.progress).show_percentage());
                        ui.add_space(4.0);
                        ui.label("正在处理...");
                    } else {
                        ui.label(
                            egui::RichText::new("等待开始...").color(tc.text_secondary),
                        );
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("检测统计")
                            .size(13.0)
                            .strong()
                            .color(tc.text),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("video_stats")
                        .num_columns(2)
                        .spacing([24.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("总检测目标:").color(tc.text_secondary));
                            ui.label(format!("{}", app.video_state.detected_objects));
                            ui.end_row();
                            ui.label(egui::RichText::new("平均置信度:").color(tc.text_secondary));
                            ui.label(if app.video_state.avg_confidence > 0.0 {
                                format!("{:.3}", app.video_state.avg_confidence)
                            } else {
                                "-".to_string()
                            });
                            ui.end_row();
                            ui.label(egui::RichText::new("处理帧率:").color(tc.text_secondary));
                            ui.label(if app.video_state.fps > 0.0 {
                                format!("{:.1}", app.video_state.fps)
                            } else {
                                "-".to_string()
                            });
                            ui.end_row();
                            ui.label(egui::RichText::new("输出文件:").color(tc.text_secondary));
                            ui.label(app.video_state.output_dir.as_deref().unwrap_or("不保存"));
                            ui.end_row();
                        });
                });
            },
        );
    });

    if need_restart {
        start_video_inference(app);
    }
}

fn config_row(ui: &mut egui::Ui, label: &str, mut add_control: impl FnMut(&mut egui::Ui), tc: &crate::theme::ThemeColors) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [50.0, 20.0],
            egui::Label::new(
                egui::RichText::new(label).color(tc.text_secondary).size(13.0),
            ),
        );
        add_control(ui);
    });
}

fn start_video_inference(app: &mut RustToolsApp) {
    let video_path = app.video_state.video_path.clone().unwrap_or_default();
    let model_idx = app.video_state.model_idx;
    let conf = app.video_state.confidence;
    let iou = app.video_state.iou_threshold;
    let _output_dir = app.video_state.output_dir.clone();

    app.video_state.is_processing = true;
    app.video_state.progress = 0.0;
    app.video_state.current_frame = 0;
    app.video_state.detected_objects = 0;
    app.video_state.status_message = Some("启动推理...".to_string());

    let (tx, rx) = std::sync::mpsc::channel::<InferenceEvent>();
    app.video_state.inference_rx = Some(rx);

    app.video_state.last_model_idx = model_idx;
    app.video_state.last_confidence = conf;
    app.video_state.last_iou_threshold = iou;

    let model = VIDEO_MODELS.get(model_idx).unwrap_or(&VIDEO_MODELS[0]).to_string();
    let python = crate::services::python_env::resolved_python().unwrap_or_else(|| "python3".to_string());

    let (kill_tx, kill_rx) = std::sync::mpsc::channel::<()>();
    app.video_state.child_kill_tx = Some(kill_tx);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async {
            let mut cmd = tokio::process::Command::new(&python);
            cmd.arg("-c");
            let script = format!(
                r#"
import sys, json, base64
from io import BytesIO
sys.stdout.reconfigure(line_buffering=True)
from ultralytics import YOLO
from PIL import Image
model = YOLO('{}')
kwargs = {{'conf': {}, 'iou': {}, 'save': False, 'stream': True, 'verbose': False}}
frame_count = 0
total_dets = 0
for result in model('{}', **kwargs):
    frame_count += 1
    dets = len(result.boxes) if result.boxes is not None else 0
    total_dets += dets
    plotted = result.plot()
    img = Image.fromarray(plotted)
    img = img.convert('RGB')
    buf = BytesIO()
    img.save(buf, format='JPEG', quality=75)
    img_b64 = base64.b64encode(buf.getvalue()).decode('utf-8')
    boxes = []
    if result.boxes is not None:
        for box in result.boxes:
            x1, y1, x2, y2 = box.xyxy[0].tolist()
            cls = int(box.cls[0])
            conf = float(box.conf[0])
            boxes.append({{'x1': x1, 'y1': y1, 'x2': x2, 'y2': y2, 'cls': cls, 'conf': conf}})
    print(json.dumps({{'type': 'frame', 'frame': frame_count, 'image': img_b64, 'detections': boxes}}), flush=True)
    if frame_count % 10 == 0:
        print(json.dumps({{'type': 'progress', 'frame': frame_count, 'detections': total_dets}}), flush=True)
print(json.dumps({{'type': 'complete', 'total_frames': frame_count, 'total_detections': total_dets}}), flush=True)
"#,
                model, conf, iou, video_path
            );
            cmd.arg(&script);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        loop {
                            // 定期检查终止信号（100ms 超时）
                            match tokio::time::timeout(
                                std::time::Duration::from_millis(100),
                                lines.next_line()
                            ).await {
                                Ok(Ok(Some(line))) => {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                                        match val.get("type").and_then(|v| v.as_str()) {
                                            Some("progress") => {
                                                let frame = val.get("frame").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let dets = val.get("detections").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                                let _ = tx.send(InferenceEvent::Progress { frame, detections: dets });
                                            }
                                            Some("frame") => {
                                                let frame = val.get("frame").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let image_b64 = val.get("image").and_then(|v| v.as_str()).unwrap_or("");
                                                let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, image_b64).unwrap_or_default();
                                                let detections: Vec<DetectionResult> = val.get("detections")
                                                    .and_then(|v| v.as_array())
                                                    .map(|arr| {
                                                        arr.iter().filter_map(|item| {
                                                            serde_json::from_value(item.clone()).ok()
                                                        }).collect()
                                                    }).unwrap_or_default();
                                                let _ = tx.send(InferenceEvent::Frame { image_bytes, detections, frame });
                                            }
                                            Some("complete") => {
                                                let frames = val.get("total_frames").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let dets = val.get("total_detections").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                                let _ = tx.send(InferenceEvent::Complete { total_frames: frames, total_detections: dets });
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Ok(Ok(None)) => break,
                                Ok(Err(_)) => break,
                                Err(_) => {
                                    // timeout: 检查终止信号
                                    if kill_rx.try_recv().is_ok() {
                                        let _ = child.kill().await;
                                        let _ = child.wait().await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    let _ = child.wait().await;
                }
                Err(e) => {
                    let _ = tx.send(InferenceEvent::Error(format!("启动失败: {}", e)));
                }
            }
        });
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
