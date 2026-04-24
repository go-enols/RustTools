use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::services::python_env::{PythonEnvStatus, get_env_status};
use crate::services::trainer::TrainerService;
use crate::ui::settings::SettingsState;
use crate::ui::annotation::AnnotationState;
use crate::ui::training::TrainingPageState;
use crate::ui::video::VideoPageState;
use crate::ui::desktop::DesktopPageState;
use crate::ui::device::DeviceInfoState;

pub use crate::route::Route;

/// 全局应用状态
pub struct RustToolsApp {
    pub route: Route,
    pub python_env_status: PythonEnvStatus,
    #[allow(dead_code)]
    pub tokio_rt: Arc<Runtime>,
    #[allow(dead_code)]
    pub toast_messages: Vec<(String, f32)>, // (message, time_remaining)
    /// 最近一次操作错误消息（由 UI 消费后清空）
    pub last_error: Option<String>,
    pub settings_state: SettingsState,
    /// 当前打开的项目配置
    pub current_project: Option<crate::models::ProjectConfig>,
    pub annotation_state: AnnotationState,
    pub training_state: TrainingPageState,
    pub video_state: VideoPageState,
    pub desktop_state: DesktopPageState,
    pub device_info_state: DeviceInfoState,
    #[allow(dead_code)]
    pub trainer_service: TrainerService,
    pub current_training_id: Option<String>,
    pub dark_mode: bool,
}

impl RustToolsApp {
    /// 获取当前主题颜色
    pub fn colors(&self) -> crate::theme::ThemeColors {
        if self.dark_mode {
            crate::theme::ThemeColors::dark()
        } else {
            crate::theme::ThemeColors::light()
        }
    }
}

impl RustToolsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_cjk_fonts(cc);
        crate::theme::apply_light_theme(&cc.egui_ctx);

        let tokio_rt = Arc::new(
            Runtime::new().expect("创建 Tokio 运行时失败")
        );
        let trainer_service = TrainerService::new();

        let python_env_status = get_env_status();

        Self {
            route: Route::Welcome,
            python_env_status,
            tokio_rt,
            toast_messages: Vec::new(),
            last_error: None,
            settings_state: SettingsState::default(),
            current_project: None,
            annotation_state: AnnotationState::default(),
            training_state: TrainingPageState::default(),
            video_state: VideoPageState::default(),
            desktop_state: DesktopPageState::default(),
            device_info_state: DeviceInfoState::default(),
            trainer_service,
            current_training_id: None,
            dark_mode: false,
        }
    }

    /// 加载字体到 egui - 统一使用 Noto Sans CJK SC，不混用其他字体
    fn setup_cjk_fonts(cc: &eframe::CreationContext<'_>) {
        let mut fonts = egui::FontDefinitions::default();

        // 清除默认字体数据，只保留 Noto Sans CJK SC
        fonts.font_data.clear();
        fonts.families.clear();

        // 嵌入的 Noto Sans CJK SC Regular（约 16MB，覆盖中文/英文/数字/基本符号）
        const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf");
        let font_data = egui::FontData::from_owned(EMBEDDED_FONT.to_vec());
        fonts.font_data.insert("noto_cjk".to_owned(), std::sync::Arc::new(font_data));

        // 所有字族统一使用 Noto Sans CJK SC
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.insert(family, vec!["noto_cjk".to_owned()]);
        }

        cc.egui_ctx.set_fonts(fonts);
    }

    fn show_nav_button(&mut self, ui: &mut egui::Ui, route: Route) {
        use crate::theme::{AppleColors, DarkColors, module_color};
        let selected = self.route == route;
        let brand = module_color(route);
        let (bg_deep, surface, text, text_secondary) = if self.dark_mode {
            (DarkColors::BG_DEEP, DarkColors::SURFACE, DarkColors::TEXT, DarkColors::TEXT_SECONDARY)
        } else {
            (AppleColors::BG_DEEP, AppleColors::SURFACE, AppleColors::TEXT, AppleColors::TEXT_SECONDARY)
        };

        let available_w = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(available_w, 44.0),
            egui::Sense::click(),
        );
        let hovered = response.hovered();
        let painter = ui.painter();

        // 背景：选中时品牌色微透，悬停时浅灰
        if selected {
            painter.rect_filled(rect, egui::CornerRadius::same(10), brand.gamma_multiply(0.08));
        } else if hovered {
            painter.rect_filled(rect, egui::CornerRadius::same(10), bg_deep);
        }

        // 左侧指示条：选中或悬停时显示
        if selected || hovered {
            let indicator_w = 3.0;
            let indicator_h = if selected { rect.height() * 0.6 } else { rect.height() * 0.4 };
            let indicator = egui::Rect::from_center_size(
                egui::pos2(rect.min.x + indicator_w * 0.5 + 2.0, rect.center().y),
                egui::vec2(indicator_w, indicator_h),
            );
            painter.rect_filled(indicator, egui::CornerRadius::same(2), brand);
        }

        // 图标圆形
        let icon_size = 28.0;
        let icon_rect = egui::Rect::from_center_size(
            rect.min + egui::vec2(16.0 + icon_size * 0.5, rect.height() * 0.5),
            egui::vec2(icon_size, icon_size),
        );
        let icon_bg = if selected { brand } else { brand.gamma_multiply(0.12) };
        painter.circle_filled(icon_rect.center(), icon_size * 0.5, icon_bg);

        // 绘制几何图标
        let icon_color = if selected { surface } else { brand };
        crate::ui::icons::draw_nav_icon(painter, icon_rect.shrink(6.0), route, icon_color);

        // 文字
        let text_pos = rect.min + egui::vec2(icon_size + 28.0, rect.height() * 0.5);
        let text_color = if selected { text } else { text_secondary };
        painter.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            &route.to_string(),
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
            text_color,
        );

        if response.clicked() {
            self.route = route;
        }
    }
}

impl eframe::App for RustToolsApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 桌面捕获实时预览已移至 desktop.rs 的 show() 中，不再使用独立 overlay viewport
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        use crate::theme::{AppleColors, DarkColors};
        crate::theme::toggle_theme(ctx, self.dark_mode);

        let (bg, surface, text, text_secondary) = if self.dark_mode {
            (DarkColors::BG, DarkColors::SURFACE, DarkColors::TEXT, DarkColors::TEXT_SECONDARY)
        } else {
            (AppleColors::BG, AppleColors::SURFACE, AppleColors::TEXT, AppleColors::TEXT_SECONDARY)
        };

        if self.route == Route::Welcome {
            // ── 欢迎页：全屏独立展示，无侧边栏 ──
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(bg)
                        .inner_margin(egui::Margin::same(0)),
                )
                .show_inside(ui, |ui| {
                    crate::ui::welcome::show(ui, self);
                });
        } else {
            // ── 左侧导航栏 ──
            egui::Panel::left("nav_panel")
                .exact_size(200.0)
                .resizable(false)
                .frame(egui::Frame::new().fill(surface))
                .show_inside(ui, |ui| {
                    ui.add_space(20.0);
                    // App logo / title
                    ui.horizontal(|ui| {
                        let logo_size = 32.0;
                        let (logo_rect, _) = ui.allocate_exact_size(egui::vec2(logo_size, logo_size), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(logo_rect, egui::CornerRadius::same(8), AppleColors::PRIMARY);
                        let inner = logo_rect.shrink(logo_size * 0.3);
                        painter.rect_filled(inner, egui::CornerRadius::same(3), surface);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("RustTools").size(16.0).strong().color(text));
                            ui.label(egui::RichText::new("开发工具箱").size(11.0).color(text_secondary));
                        });
                    });
                    ui.add_space(24.0);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        self.show_nav_button(ui, Route::Hub);
                        self.show_nav_button(ui, Route::Project);
                        self.show_nav_button(ui, Route::Annotation);
                        self.show_nav_button(ui, Route::Training);
                        self.show_nav_button(ui, Route::Video);
                        self.show_nav_button(ui, Route::Desktop);
                        self.show_nav_button(ui, Route::Device);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        self.show_nav_button(ui, Route::Settings);
                    });

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        // 主题切换按钮
                        ui.horizontal(|ui| {
                            let theme_btn = if self.dark_mode { "☀ 浅色" } else { "🌙 深色" };
                            if ui.button(egui::RichText::new(theme_btn).size(11.0)).clicked() {
                                self.dark_mode = !self.dark_mode;
                            }
                        });
                        ui.add_space(4.0);
                        let env_ok = self.python_env_status.python_available;
                        ui.horizontal(|ui| {
                            crate::theme::status_indicator(ui, env_ok, if env_ok { "环境就绪" } else { "环境未就绪" });
                        });
                        ui.add_space(8.0);
                    });
                });

            // ── 中央内容区 ──
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(bg)
                        .inner_margin(egui::Margin::same(20)),
                )
                .show_inside(ui, |ui| {
                    match self.route {
                        Route::Welcome => crate::ui::welcome::show(ui, self),
                        Route::Hub => crate::ui::hub::show(ui, self),
                        Route::Project => crate::ui::project::show(ui, self),
                        Route::Annotation => crate::ui::annotation::show(ui, self),
                        Route::Training => crate::ui::training::show(ui, self),
                        Route::Video => crate::ui::video::show(ui, self),
                        Route::Desktop => crate::ui::desktop::show(ui, self, Some(frame)),
                        Route::Device => crate::ui::device::show(ui, self),
                        Route::Settings => crate::ui::settings::show(ui, self),
                    }
                });
        }
    }
}
