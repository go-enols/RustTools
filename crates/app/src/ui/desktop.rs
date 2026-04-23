use eframe::egui;
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::app::RustToolsApp;
use crate::theme::{AppleColors, compact_card_frame, page_header};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DetectionResult {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    #[serde(rename = "cls")]
    pub class_id: usize,
    #[serde(rename = "conf")]
    pub confidence: f32,
}

/// 捕获区域（屏幕坐标）
#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 共享给主 UI 和后台线程的状态
pub struct OverlayState {
    pub is_capturing: bool,
    /// 检测框通过 Arc 共享，避免每帧 clone Vec
    pub detections: std::sync::Arc<Vec<DetectionResult>>,
    pub fps: f32,
    pub inference_ms: f32,
    pub detected_objects: usize,
    pub capture_region: CaptureRegion,
    /// 最新一帧画面（已转为 egui Color32，Arc 避免 Mutex 内拷贝）
    pub frame_colors: Option<(std::sync::Arc<Vec<egui::Color32>>, [usize; 2])>,
    /// 帧版本号，主 UI 只在版本变化时更新 texture，避免重复 GPU 上传
    pub frame_version: u64,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            is_capturing: false,
            detections: std::sync::Arc::new(Vec::new()),
            fps: 0.0,
            inference_ms: 0.0,
            detected_objects: 0,
            capture_region: CaptureRegion::default(),
            frame_colors: None,
            frame_version: 0,
        }
    }
}

#[derive(Clone)]
pub struct CaptureSettings {
    pub model: String,
    pub conf: f32,
}

/// 桌面捕获页面状态
pub struct DesktopPageState {
    pub capture_source: CaptureSource,
    pub model_idx: usize,
    pub confidence: f32,
    pub show_fps: bool,
    pub show_boxes: bool,
    pub show_labels: bool,
    pub save_video: bool,
    pub is_capturing: bool,
    pub settings: Arc<Mutex<CaptureSettings>>,
    pub overlay_state: Arc<Mutex<OverlayState>>,
    pub capture_region: Arc<Mutex<CaptureRegion>>,
    /// 后台线程停止标志，点击「停止捕获」时设为 false
    pub capture_running: Arc<AtomicBool>,
    /// 最新一帧的 egui texture（供主 UI 显示）
    pub last_frame: Option<egui::TextureHandle>,
    /// 上次更新的 frame_version，避免重复 texture.set()
    pub last_frame_version: u64,
}

impl Default for DesktopPageState {
    fn default() -> Self {
        Self {
            capture_source: CaptureSource::FullScreen,
            model_idx: 0,
            confidence: 0.25,
            show_fps: true,
            show_boxes: true,
            show_labels: true,
            save_video: false,
            is_capturing: false,
            settings: Arc::new(Mutex::new(CaptureSettings {
                model: DESKTOP_MODELS[0].to_string(),
                conf: 0.25,
            })),
            overlay_state: Arc::new(Mutex::new(OverlayState::default())),
            capture_region: Arc::new(Mutex::new(CaptureRegion::default())),
            capture_running: Arc::new(AtomicBool::new(false)),
            last_frame: None,
            last_frame_version: 0,
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum CaptureSource {
    #[default]
    FullScreen,
    PrimaryMonitor,
    SecondaryMonitor,
    Window,
    Region,
}

impl CaptureSource {
    pub fn label(&self) -> &'static str {
        match self {
            CaptureSource::FullScreen => "全屏捕获",
            CaptureSource::PrimaryMonitor => "主显示器",
            CaptureSource::SecondaryMonitor => "副显示器",
            CaptureSource::Window => "指定窗口",
            CaptureSource::Region => "选择区域",
        }
    }
}

const DESKTOP_MODELS: &[&str] = &["yolo11n.pt", "yolo11s.pt", "自定义模型..."];

pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let state = &mut app.desktop_state;

    // 实时同步配置到共享状态
    if state.is_capturing {
        if let Ok(mut s) = state.settings.lock() {
            s.model = DESKTOP_MODELS.get(state.model_idx).unwrap_or(&DESKTOP_MODELS[0]).to_string();
            s.conf = state.confidence;
        }
    }

    // 从 overlay_state 读取最新统计
    let (fps, inference_ms, detected_objects) = {
        let os = state.overlay_state.lock().unwrap();
        (os.fps, os.inference_ms, os.detected_objects)
    };

    page_header(ui, "桌面捕获", "实时屏幕捕获与目标检测");

    let available = ui.available_size();
    let left_w = (available.x * 0.28).min(280.0);
    let right_w = available.x - left_w - 16.0;

    ui.horizontal_top(|ui| {
        // ── 左侧：控制面板 ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("desktop_control_scroll")
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("控制面板")
                                    .size(14.0)
                                    .strong()
                                    .color(AppleColors::TEXT),
                            );
                            ui.add_space(12.0);

                            ui.label(
                                egui::RichText::new("捕获源").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.add_space(4.0);
                            for s in [
                                CaptureSource::FullScreen,
                                CaptureSource::PrimaryMonitor,
                                CaptureSource::SecondaryMonitor,
                                CaptureSource::Window,
                                CaptureSource::Region,
                            ] {
                                if ui.selectable_label(state.capture_source == s, s.label()).clicked() {
                                    state.capture_source = s;
                                }
                            }
                            ui.add_space(10.0);

                            ui.label(
                                egui::RichText::new("检测模型").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.add_space(4.0);
                            let model_name = DESKTOP_MODELS.get(state.model_idx).unwrap_or(&DESKTOP_MODELS[0]);
                            egui::ComboBox::from_id_salt("desktop_model")
                                .selected_text(*model_name)
                                .width(left_w - 32.0)
                                .show_ui(ui, |ui| {
                                    for (i, name) in DESKTOP_MODELS.iter().enumerate() {
                                        ui.selectable_value(&mut state.model_idx, i, *name);
                                    }
                                });
                            ui.add_space(10.0);

                            ui.label(
                                egui::RichText::new("置信度阈值").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.add_space(4.0);
                            ui.add(egui::Slider::new(&mut state.confidence, 0.01..=1.0).show_value(true));
                            ui.add_space(10.0);

                            ui.checkbox(&mut state.show_fps, "显示 FPS");
                            ui.checkbox(&mut state.show_boxes, "显示检测框");
                            ui.checkbox(&mut state.show_labels, "显示标签");
                            ui.checkbox(&mut state.save_video, "保存检测视频");
                            ui.add_space(10.0);

                            // ── 捕获区域设置 ──
                            ui.label(
                                egui::RichText::new("捕获区域").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.add_space(4.0);
                            {
                                let mut cr = state.capture_region.lock().unwrap();
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    ui.add(egui::DragValue::new(&mut cr.x).speed(10));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Y:");
                                    ui.add(egui::DragValue::new(&mut cr.y).speed(10));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("宽:");
                                    ui.add(egui::DragValue::new(&mut cr.width).speed(10).clamp_range(100..=8192));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("高:");
                                    ui.add(egui::DragValue::new(&mut cr.height).speed(10).clamp_range(100..=8192));
                                });
                            }
                            ui.add_space(10.0);

                            ui.label(
                                egui::RichText::new("计算设备").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.horizontal(|ui| {
                                ui.label("设备:");
                                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 3.5, AppleColors::TEXT_SECONDARY);
                                ui.label("CPU (Rust/ONNX Runtime)");
                            });

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new("实时统计").size(12.0).strong().color(AppleColors::TEXT),
                            );
                            ui.add_space(4.0);
                            egui::Grid::new("desktop_stats")
                                .num_columns(2)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("帧率:").color(AppleColors::TEXT_SECONDARY));
                                    ui.label(if fps > 0.0 {
                                        format!("{:.1} FPS", fps)
                                    } else {
                                        "- FPS".to_string()
                                    });
                                    ui.end_row();
                                    ui.label(egui::RichText::new("检测目标:").color(AppleColors::TEXT_SECONDARY));
                                    ui.label(format!("{}", detected_objects));
                                    ui.end_row();
                                    ui.label(egui::RichText::new("推理耗时:").color(AppleColors::TEXT_SECONDARY));
                                    ui.label(if inference_ms > 0.0 {
                                        format!("{:.1} ms", inference_ms)
                                    } else {
                                        "- ms".to_string()
                                    });
                                    ui.end_row();
                                });

                            ui.add_space(12.0);

                            let can_start = !state.is_capturing;

                            if !state.is_capturing {
                                let start_btn = egui::Button::new(
                                    egui::RichText::new("开始捕获").color(AppleColors::SURFACE).strong(),
                                )
                                .fill(AppleColors::SUCCESS)
                                .corner_radius(egui::CornerRadius::same(8));
                                if ui.add_sized([ui.available_width(), 40.0], start_btn).clicked() && can_start {
                                    start_capture(state);
                                }
                            } else {
                                let stop_btn = egui::Button::new(
                                    egui::RichText::new("停止捕获").color(AppleColors::SURFACE).strong(),
                                )
                                .fill(AppleColors::DANGER)
                                .corner_radius(egui::CornerRadius::same(8));
                                if ui.add_sized([ui.available_width(), 40.0], stop_btn).clicked() {
                                    state.capture_running.store(false, Ordering::Relaxed);
                                    state.is_capturing = false;
                                }
                            }
                        });
                });
            },
        );

        ui.add_space(8.0);

        // ── 右侧：实时预览 ──
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available.y),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                compact_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("实时预览")
                                .size(14.0)
                                .strong()
                                .color(AppleColors::TEXT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if state.is_capturing {
                                ui.horizontal(|ui| {
                                    let (dot_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(8.0, 8.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(dot_rect.center(), 4.0, AppleColors::DANGER);
                                    ui.label(
                                        egui::RichText::new("录制中")
                                            .size(12.0)
                                            .color(AppleColors::DANGER),
                                    );
                                });
                            } else {
                                ui.label(
                                    egui::RichText::new("已停止")
                                        .size(12.0)
                                        .color(AppleColors::TEXT_SECONDARY),
                                );
                            }
                        });
                    });
                    ui.add_space(8.0);

                    let preview_rect = ui.available_rect_before_wrap();

                    // 尝试读取最新帧
                    let (frame_opt, detections, region, frame_version) = {
                        let os = state.overlay_state.lock().unwrap();
                        (os.frame_colors.clone(), os.detections.clone(), os.capture_region, os.frame_version)
                    };

                    if let Some((colors, [fw, fh])) = frame_opt {
                        // 获取或创建 texture
                        let texture = state.last_frame.get_or_insert_with(|| {
                            ui.ctx().load_texture(
                                "desktop_preview",
                                egui::ColorImage::example(),
                                egui::TextureOptions::LINEAR,
                            )
                        });
                        // 只在 frame_version 变化时更新 texture，避免重复 GPU 上传
                        if frame_version != state.last_frame_version {
                            let color_image = egui::ColorImage {
                                size: [fw, fh],
                                source_size: egui::vec2(fw as f32, fh as f32),
                                pixels: (*colors).clone(),
                            };
                            texture.set(color_image, egui::TextureOptions::LINEAR);
                            state.last_frame_version = frame_version;
                        }

                        // 等比缩放显示
                        let fw_f = fw as f32;
                        let fh_f = fh as f32;
                        let img_aspect = fw_f / fh_f.max(1.0);
                        let rect_aspect = preview_rect.width() / preview_rect.height().max(1.0);
                        let display_rect = if img_aspect > rect_aspect {
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

                        // 绘制画面
                        ui.painter().image(
                            texture.id(),
                            display_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );

                        // 绘制检测框（映射到显示区域坐标）
                        if state.show_boxes {
                            let scale_x = display_rect.width() / fw_f;
                            let scale_y = display_rect.height() / fh_f;
                            for det in detections.iter() {
                                let det_rect = egui::Rect::from_min_max(
                                    egui::pos2(
                                        display_rect.min.x + det.x1 * scale_x,
                                        display_rect.min.y + det.y1 * scale_y,
                                    ),
                                    egui::pos2(
                                        display_rect.min.x + det.x2 * scale_x,
                                        display_rect.min.y + det.y2 * scale_y,
                                    ),
                                );
                                ui.painter().rect_stroke(
                                    det_rect,
                                    egui::CornerRadius::same(2),
                                    egui::Stroke::new(2.0, class_color(det.class_id)),
                                    egui::StrokeKind::Inside,
                                );
                                if state.show_labels {
                                    ui.painter().text(
                                        det_rect.min + egui::vec2(2.0, 12.0),
                                        egui::Align2::LEFT_TOP,
                                        &format!("{:.0}%", det.confidence * 100.0),
                                        egui::FontId::new(11.0, egui::FontFamily::Proportional),
                                        class_color(det.class_id),
                                    );
                                }
                            }
                        }

                        // 绘制捕获区域边框（相对画面）
                        if region.width > 0 && region.height > 0 {
                            let border_rect = egui::Rect::from_min_max(
                                egui::pos2(display_rect.min.x, display_rect.min.y),
                                egui::pos2(display_rect.max.x, display_rect.max.y),
                            );
                            ui.painter().rect_stroke(
                                border_rect,
                                egui::CornerRadius::same(0),
                                egui::Stroke::new(2.0, egui::Color32::RED),
                                egui::StrokeKind::Inside,
                            );
                        }
                    } else {
                        // 无画面时显示占位
                        ui.painter().rect_filled(
                            preview_rect,
                            egui::CornerRadius::same(8),
                            AppleColors::BG_DEEP,
                        );
                        ui.painter().text(
                            preview_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            if state.is_capturing {
                                "正在初始化捕获…"
                            } else {
                                "点击「开始捕获」以启动实时检测"
                            },
                            egui::FontId::new(14.0, egui::FontFamily::Proportional),
                            AppleColors::TEXT_SECONDARY,
                        );
                    }
                });
            },
        );
    });
}



fn start_capture(state: &mut DesktopPageState) {
    state.is_capturing = true;
    state.capture_running.store(true, Ordering::Relaxed);

    let overlay_state = Arc::clone(&state.overlay_state);
    let settings = Arc::clone(&state.settings);
    let capture_region = Arc::clone(&state.capture_region);
    let capture_running = Arc::clone(&state.capture_running);

    std::thread::spawn(move || {
        // 1. 从配置读取模型名并推导 ONNX 路径
        let model_name = {
            let s = settings.lock().unwrap();
            s.model.clone()
        };
        let onnx_name = if model_name.ends_with(".pt") {
            format!("{}.onnx", &model_name[..model_name.len() - 3])
        } else if model_name.ends_with(".onnx") {
            model_name
        } else {
            format!("{}.onnx", model_name)
        };

        // 2. 加载 ONNX 模型（一次）
        let engine = match crate::services::yolo_onnx::YoloOnnxEngine::new(
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join(&onnx_name)))
                .filter(|p| p.exists())
                .or_else(|| Some(std::path::PathBuf::from(&onnx_name)))
                .unwrap()
                .to_string_lossy()
                .as_ref()
        ) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("[Desktop] ONNX 模型加载失败: {}", err);
                return;
            }
        };

        // 3. 初始化 scrap 捕获器
        let (mut capturer, screen_width, screen_height) = match init_scrap_capturer() {
            Some((c, w, h)) => (c, w, h),
            None => {
                eprintln!("[Desktop] 屏幕捕获器初始化失败");
                return;
            }
        };

        // 更新默认捕获区域为全屏
        {
            let mut cr = capture_region.lock().unwrap();
            cr.x = 0;
            cr.y = 0;
            cr.width = screen_width;
            cr.height = screen_height;
        }

        // 预热几帧
        for _ in 0..3 {
            let _ = capturer.frame();
        }

        // 4. 使用 mpsc channel 替代 Arc<Mutex>，减少锁竞争与忙等待
        let (infer_input_tx, infer_input_rx) = std::sync::mpsc::channel::<(Vec<u8>, u32, u32)>();
        let (infer_output_tx, infer_output_rx) = std::sync::mpsc::channel::<(std::sync::Arc<Vec<DetectionResult>>, f32)>();
        let infer_conf: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.25));

        // 5. 启动推理子线程
        let infer_conf_c = Arc::clone(&infer_conf);
        let infer_running_c = Arc::clone(&capture_running);
        let infer_handle = std::thread::spawn(move || {
            let mut engine = engine;
            let mut last_conf = 0.0f32;
            while infer_running_c.load(Ordering::Relaxed) {
                // 实时更新置信度
                let conf = *infer_conf_c.lock().unwrap();
                if (conf - last_conf).abs() > 0.001 {
                    engine.set_conf_threshold(conf);
                    last_conf = conf;
                }

                match infer_input_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                    Ok((bgra, w, h)) => {
                        let t = Instant::now();
                        let dets = match engine.infer_from_bgra(&bgra, w, h) {
                            Ok(d) => d,
                            Err(e) => {
                                eprintln!("[Desktop-推理线程] 推理失败: {}", e);
                                Vec::new()
                            }
                        };
                        let ms = t.elapsed().as_secs_f32() * 1000.0;

                        let mapped: std::sync::Arc<Vec<DetectionResult>> = std::sync::Arc::new(
                            dets.into_iter()
                                .map(|d| DetectionResult {
                                    x1: d.x1,
                                    y1: d.y1,
                                    x2: d.x2,
                                    y2: d.y2,
                                    class_id: d.class_id,
                                    confidence: d.confidence,
                                })
                                .collect()
                        );

                        let _ = infer_output_tx.send((mapped, ms));
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        // 6. 截屏循环（主后台线程）
        let mut fps = 0.0f32;
        let target_interval = std::time::Duration::from_millis(16);

        while capture_running.load(Ordering::Relaxed) {
            let loop_start = Instant::now();

            // 同步置信度到推理线程
            let conf = {
                let s = settings.lock().unwrap();
                s.conf
            };
            *infer_conf.lock().unwrap() = conf;

            // 读取当前捕获区域
            let region = { *capture_region.lock().unwrap() };

            // scrap 截屏（BGRA 格式），直接从 Frame 的 slice 裁剪，避免 to_vec() 拷贝
            let cropped = match capturer.frame() {
                Ok(f) => {
                    let data: &[u8] = &f;
                    crop_bgra(data, screen_width, screen_height, &region)
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
            };

            // BGRA → egui Color32 供主 UI 直接显示（截屏线程预转换，主 UI 零开销）
            let preview_colors = bgra_to_color32(&cropped);
            let preview_size = [region.width as usize, region.height as usize];

            // 提交 BGRA 给推理线程（mpsc 无锁）
            let _ = infer_input_tx.send((cropped, region.width, region.height));

            // 获取最新检测框与推理耗时（try_recv 所有结果，只保留最新的）
            let mut detections: std::sync::Arc<Vec<DetectionResult>> = std::sync::Arc::new(Vec::new());
            let mut inference_ms = 0.0f32;
            while let Ok((dets, ms)) = infer_output_rx.try_recv() {
                detections = dets;
                inference_ms = ms;
            }

            // 计算 FPS：使用指数移动平均平滑单帧波动
            let frame_time_ms = loop_start.elapsed().as_secs_f32() * 1000.0;
            let instant_fps = 1000.0 / frame_time_ms.max(1.0);
            fps = fps * 0.8 + instant_fps * 0.2;

            // 更新 overlay 状态（供主 UI 显示）
            {
                let mut os = overlay_state.lock().unwrap();
                os.is_capturing = true;
                os.detections = detections;
                os.fps = fps;
                os.inference_ms = inference_ms;
                os.detected_objects = os.detections.len();
                os.capture_region = region;
                os.frame_colors = Some((std::sync::Arc::new(preview_colors), preview_size));
                os.frame_version += 1;
            }

            // 帧率控制
            let elapsed = loop_start.elapsed();
            if elapsed < target_interval {
                std::thread::sleep(target_interval - elapsed);
            }
        }

        // 通知推理线程退出并等待
        capture_running.store(false, Ordering::Relaxed);
        let _ = infer_handle.join();

        // 标记 overlay 状态为停止
        {
            let mut os = overlay_state.lock().unwrap();
            os.is_capturing = false;
            os.detections = std::sync::Arc::new(Vec::new());
        }
    });
}

pub fn class_color(class_id: usize) -> egui::Color32 {
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

#[allow(dead_code)]
fn parse_detections(stdout: &str) -> Vec<DetectionResult> {
    let mut results = Vec::new();
    for line in stdout.lines() {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(line) {
            for item in arr {
                if let Ok(det) = serde_json::from_value::<DetectionResult>(item.clone()) {
                    results.push(det);
                }
            }
        }
    }
    results
}

/// 使用 scrap 初始化屏幕捕获器
fn init_scrap_capturer() -> Option<(scrap::Capturer, u32, u32)> {
    let displays = scrap::Display::all().ok()?;
    let display = displays.into_iter().next()?;
    let width = display.width() as u32;
    let height = display.height() as u32;
    let capturer = scrap::Capturer::new(display).ok()?;
    Some((capturer, width, height))
}

/// 从全屏 BGRA 数据中裁剪出指定区域
fn crop_bgra(src: &[u8], src_w: u32, src_h: u32, region: &CaptureRegion) -> Vec<u8> {
    let x = region.x.max(0) as u32;
    let y = region.y.max(0) as u32;
    let w = region.width.min(src_w.saturating_sub(x));
    let h = region.height.min(src_h.saturating_sub(y));

    if w == 0 || h == 0 || src.len() < (src_w * src_h * 4) as usize {
        return Vec::new();
    }

    let mut dst = vec![0u8; (w * h * 4) as usize];
    for row in 0..h {
        let src_start = ((y + row) * src_w + x) * 4;
        let dst_start = row * w * 4;
        dst[dst_start as usize..(dst_start + w * 4) as usize]
            .copy_from_slice(&src[src_start as usize..(src_start + w * 4) as usize]);
    }
    dst
}

/// BGRA → RGBA，同时强制 Alpha = 255（解决 X11 scrap alpha=0 问题）
fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(255);      // A
    }
    rgba
}

/// BGRA → Vec<egui::Color32>，截屏线程预转换，主 UI 直接创建 ColorImage 零开销
fn bgra_to_color32(bgra: &[u8]) -> Vec<egui::Color32> {
    let mut colors = Vec::with_capacity(bgra.len() / 4);
    for chunk in bgra.chunks_exact(4) {
        colors.push(egui::Color32::from_rgb(chunk[2], chunk[1], chunk[0]));
    }
    colors
}
