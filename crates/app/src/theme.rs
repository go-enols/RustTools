//! Apple-inspired light theme with glassmorphism and neumorphism

use eframe::egui;

// ============================================================================
// Frame Helpers
// ============================================================================

/// Standard card container (white background, rounded corners, subtle border)
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(AppleColors::SURFACE)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .stroke(egui::Stroke::new(1.0, AppleColors::BORDER))
}

/// Compact card for tight spaces
pub fn compact_card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(AppleColors::SURFACE)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .stroke(egui::Stroke::new(1.0, AppleColors::BORDER))
}

/// Group box with title - replaces ugly default group()
pub fn titled_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    card_frame().show(ui, |ui| {
        ui.label(
            egui::RichText::new(title)
                .size(14.0)
                .strong()
                .color(AppleColors::TEXT),
        );
        ui.add_space(12.0);
        add_contents(ui);
    });
}

// ============================================================================
// Color System
// ============================================================================

pub struct AppleColors;

impl AppleColors {
    // Backgrounds
    pub const BG: egui::Color32 = egui::Color32::from_rgb(245, 245, 247);         // #F5F5F7
    pub const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(229, 229, 234);    // #E5E5EA
    pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);    // #FFFFFF
    pub const SURFACE_HOVER: egui::Color32 = egui::Color32::from_rgb(250, 250, 250);

    // Glassmorphism
    pub const GLASS: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 180);
    pub const GLASS_STRONG: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 220);

    // Brand Colors
    pub const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0, 122, 255);      // #007AFF
    pub const PRIMARY_HOVER: egui::Color32 = egui::Color32::from_rgb(0, 102, 220);
    pub const SUCCESS: egui::Color32 = egui::Color32::from_rgb(52, 199, 89);      // #34C759
    pub const WARNING: egui::Color32 = egui::Color32::from_rgb(255, 149, 0);      // #FF9500
    pub const DANGER: egui::Color32 = egui::Color32::from_rgb(255, 59, 48);       // #FF3B30
    pub const PURPLE: egui::Color32 = egui::Color32::from_rgb(175, 82, 222);      // #AF52DE
    pub const TEAL: egui::Color32 = egui::Color32::from_rgb(90, 200, 250);        // #5AC8FA
    pub const INDIGO: egui::Color32 = egui::Color32::from_rgb(88, 86, 214);       // #5856D6
    pub const PINK: egui::Color32 = egui::Color32::from_rgb(255, 55, 95);         // #FF375F

    // Text
    pub const TEXT: egui::Color32 = egui::Color32::from_rgb(29, 29, 31);          // #1D1D1F
    pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(134, 134, 139); // #86868B
    pub const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(199, 199, 204);  // #C7C7CC

    // Borders & Shadows
    pub const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 20);
    pub const SHADOW: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 15);
    pub const SHADOW_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 30);
    pub const INNER_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 200);
}

// Module brand colors
pub fn module_color(route: crate::app::Route) -> egui::Color32 {
    use crate::app::Route;
    match route {
        Route::Welcome => AppleColors::PRIMARY,
        Route::Hub => AppleColors::PRIMARY,
        Route::Project => AppleColors::INDIGO,
        Route::Annotation => AppleColors::PINK,
        Route::Training => AppleColors::SUCCESS,
        Route::Video => AppleColors::PURPLE,
        Route::Desktop => AppleColors::TEAL,
        Route::Device => AppleColors::WARNING,
        Route::Settings => AppleColors::TEXT_SECONDARY,
    }
}

pub fn module_gradient(route: crate::app::Route) -> (egui::Color32, egui::Color32) {
    use crate::app::Route;
    match route {
        Route::Welcome => (AppleColors::PRIMARY, AppleColors::TEAL),
        Route::Hub => (AppleColors::PRIMARY, AppleColors::TEAL),
        Route::Project => (AppleColors::INDIGO, AppleColors::PRIMARY),
        Route::Annotation => (AppleColors::PINK, AppleColors::WARNING),
        Route::Training => (AppleColors::SUCCESS, AppleColors::TEAL),
        Route::Video => (AppleColors::PURPLE, AppleColors::PINK),
        Route::Desktop => (AppleColors::TEAL, AppleColors::PRIMARY),
        Route::Device => (AppleColors::WARNING, AppleColors::DANGER),
        Route::Settings => (AppleColors::TEXT_SECONDARY, AppleColors::TEXT_TERTIARY),
    }
}

// ============================================================================
// Theme Colors - 统一入口，根据 dark_mode 自动切换
// ============================================================================

pub struct ThemeColors {
    pub bg: egui::Color32,
    pub bg_deep: egui::Color32,
    pub surface: egui::Color32,
    pub surface_hover: egui::Color32,
    pub glass: egui::Color32,
    pub glass_strong: egui::Color32,
    pub text: egui::Color32,
    pub text_secondary: egui::Color32,
    pub text_tertiary: egui::Color32,
    pub border: egui::Color32,
    pub shadow: egui::Color32,
    pub shadow_hover: egui::Color32,
    pub inner_highlight: egui::Color32,
}

impl ThemeColors {
    pub fn light() -> Self {
        Self {
            bg: AppleColors::BG,
            bg_deep: AppleColors::BG_DEEP,
            surface: AppleColors::SURFACE,
            surface_hover: AppleColors::SURFACE_HOVER,
            glass: AppleColors::GLASS,
            glass_strong: AppleColors::GLASS_STRONG,
            text: AppleColors::TEXT,
            text_secondary: AppleColors::TEXT_SECONDARY,
            text_tertiary: AppleColors::TEXT_TERTIARY,
            border: AppleColors::BORDER,
            shadow: AppleColors::SHADOW,
            shadow_hover: AppleColors::SHADOW_HOVER,
            inner_highlight: AppleColors::INNER_HIGHLIGHT,
        }
    }

    pub fn dark() -> Self {
        Self {
            bg: DarkColors::BG,
            bg_deep: DarkColors::BG_DEEP,
            surface: DarkColors::SURFACE,
            surface_hover: DarkColors::SURFACE_HOVER,
            glass: DarkColors::GLASS,
            glass_strong: DarkColors::GLASS_STRONG,
            text: DarkColors::TEXT,
            text_secondary: DarkColors::TEXT_SECONDARY,
            text_tertiary: DarkColors::TEXT_TERTIARY,
            border: DarkColors::BORDER,
            shadow: DarkColors::SHADOW,
            shadow_hover: DarkColors::SHADOW_HOVER,
            inner_highlight: DarkColors::INNER_HIGHLIGHT,
        }
    }
}

// ============================================================================
// Dark Color System
// ============================================================================

pub struct DarkColors;

impl DarkColors {
    pub const BG: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);               // #000000
    pub const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(28, 28, 30);       // #1C1C1E
    pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(28, 28, 30);       // #1C1C1E
    pub const SURFACE_HOVER: egui::Color32 = egui::Color32::from_rgb(44, 44, 46); // #2C2C2E
    pub const GLASS: egui::Color32 = egui::Color32::from_rgba_premultiplied(30, 30, 30, 200);
    pub const GLASS_STRONG: egui::Color32 = egui::Color32::from_rgba_premultiplied(40, 40, 40, 230);
    pub const TEXT: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);       // #FFFFFF
    pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(142, 142, 147); // #8E8E93
    pub const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(72, 72, 74);  // #48484A
    pub const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 15);
    pub const SHADOW: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 40);
    pub const SHADOW_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 60);
    pub const INNER_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 30);
}

// ============================================================================
// Theme Application
// ============================================================================

pub fn apply_light_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();

    // Core backgrounds
    visuals.panel_fill = AppleColors::BG;
    visuals.faint_bg_color = AppleColors::BG_DEEP;
    visuals.extreme_bg_color = AppleColors::BG_DEEP;
    visuals.code_bg_color = AppleColors::BG_DEEP;
    visuals.window_fill = AppleColors::GLASS;
    visuals.window_stroke = egui::Stroke::new(1.0, AppleColors::BORDER);

    // Selection & hyperlinks
    visuals.selection.bg_fill = AppleColors::PRIMARY;
    visuals.selection.stroke = egui::Stroke::new(1.0, AppleColors::SURFACE);
    visuals.hyperlink_color = AppleColors::PRIMARY;

    // Widgets - noninteractive
    visuals.widgets.noninteractive.bg_fill = AppleColors::BG_DEEP;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, AppleColors::BORDER);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, AppleColors::TEXT);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(10);

    // Widgets - inactive
    visuals.widgets.inactive.bg_fill = AppleColors::SURFACE;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, AppleColors::BORDER);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, AppleColors::TEXT);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(10);

    // Widgets - hovered
    visuals.widgets.hovered.bg_fill = AppleColors::SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, AppleColors::PRIMARY);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, AppleColors::TEXT);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.hovered.expansion = 2.0;

    // Widgets - active
    visuals.widgets.active.bg_fill = AppleColors::BG_DEEP;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, AppleColors::PRIMARY);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, AppleColors::TEXT);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(10);

    // Widgets - open (e.g. combo box dropdown)
    visuals.widgets.open.bg_fill = AppleColors::SURFACE;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, AppleColors::PRIMARY);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, AppleColors::TEXT);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(10);

    // Special widgets
    visuals.widgets.active.bg_fill = AppleColors::PRIMARY;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, AppleColors::SURFACE);

    // Rounding
    visuals.window_corner_radius = egui::CornerRadius::same(16);
    visuals.menu_corner_radius = egui::CornerRadius::same(12);
    visuals.button_frame = true;
    visuals.collapsing_header_frame = true;

    // Shadows
    visuals.window_shadow = egui::epaint::Shadow {
        color: AppleColors::SHADOW,
        offset: [0, 8],
        blur: 24,
        spread: 0,
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        color: AppleColors::SHADOW,
        offset: [0, 4],
        blur: 16,
        spread: 0,
    };

    // Text colors
    visuals.override_text_color = Some(AppleColors::TEXT);
    visuals.warn_fg_color = AppleColors::WARNING;
    visuals.error_fg_color = AppleColors::DANGER;

    ctx.set_visuals(visuals);

    // Style
    let mut style = (*ctx.global_style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style.spacing.indent = 16.0;
    style.spacing.interact_size = egui::vec2(40.0, 20.0);

    ctx.set_global_style(style);
}

pub fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Core backgrounds
    visuals.panel_fill = DarkColors::BG;
    visuals.faint_bg_color = DarkColors::BG_DEEP;
    visuals.extreme_bg_color = DarkColors::BG_DEEP;
    visuals.code_bg_color = DarkColors::BG_DEEP;
    visuals.window_fill = DarkColors::GLASS;
    visuals.window_stroke = egui::Stroke::new(1.0, DarkColors::BORDER);

    // Selection & hyperlinks
    visuals.selection.bg_fill = AppleColors::PRIMARY;
    visuals.selection.stroke = egui::Stroke::new(1.0, DarkColors::SURFACE);
    visuals.hyperlink_color = AppleColors::PRIMARY;

    // Widgets - noninteractive
    visuals.widgets.noninteractive.bg_fill = DarkColors::BG_DEEP;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, DarkColors::BORDER);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, DarkColors::TEXT);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(10);

    // Widgets - inactive
    visuals.widgets.inactive.bg_fill = DarkColors::SURFACE;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, DarkColors::BORDER);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, DarkColors::TEXT);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(10);

    // Widgets - hovered
    visuals.widgets.hovered.bg_fill = DarkColors::SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, AppleColors::PRIMARY);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, DarkColors::TEXT);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.hovered.expansion = 2.0;

    // Widgets - active
    visuals.widgets.active.bg_fill = DarkColors::BG_DEEP;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, AppleColors::PRIMARY);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, DarkColors::TEXT);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(10);

    // Widgets - open
    visuals.widgets.open.bg_fill = DarkColors::SURFACE;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, AppleColors::PRIMARY);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, DarkColors::TEXT);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(10);

    // Special widgets
    visuals.widgets.active.bg_fill = AppleColors::PRIMARY;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, DarkColors::SURFACE);

    // Rounding
    visuals.window_corner_radius = egui::CornerRadius::same(16);
    visuals.menu_corner_radius = egui::CornerRadius::same(12);
    visuals.button_frame = true;
    visuals.collapsing_header_frame = true;

    // Shadows (subtle in dark mode)
    visuals.window_shadow = egui::epaint::Shadow {
        color: DarkColors::SHADOW,
        offset: [0, 8],
        blur: 24,
        spread: 0,
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        color: DarkColors::SHADOW,
        offset: [0, 4],
        blur: 16,
        spread: 0,
    };

    // Text colors
    visuals.override_text_color = Some(DarkColors::TEXT);
    visuals.warn_fg_color = AppleColors::WARNING;
    visuals.error_fg_color = AppleColors::DANGER;

    ctx.set_visuals(visuals);

    // Style (same as light)
    let mut style = (*ctx.global_style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style.spacing.indent = 16.0;
    style.spacing.interact_size = egui::vec2(40.0, 20.0);

    ctx.set_global_style(style);
}

/// 切换主题并应用到上下文
pub fn toggle_theme(ctx: &egui::Context, dark: bool) {
    if dark {
        apply_dark_theme(ctx);
    } else {
        apply_light_theme(ctx);
    }
}

// ============================================================================
// Shared Widget Helpers
// ============================================================================

/// Glassmorphism card background
pub fn glass_card(ui: &mut egui::Ui, rect: egui::Rect, corner_radius: f32) {
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(corner_radius as u8),
        AppleColors::GLASS,
    );
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(corner_radius as u8),
        egui::Stroke::new(1.0, AppleColors::BORDER),
        egui::StrokeKind::Inside,
    );
}

/// Neumorphism-style card with soft shadow
pub fn neumorphic_card(ui: &mut egui::Ui, rect: egui::Rect, response: &egui::Response, corner_radius: f32) {
    let bg = if response.hovered() {
        AppleColors::SURFACE_HOVER
    } else {
        AppleColors::SURFACE
    };

    // Main background
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(corner_radius as u8),
        bg,
    );

    // Subtle border
    let border_color = if response.hovered() {
        AppleColors::PRIMARY
    } else {
        AppleColors::BORDER
    };
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(corner_radius as u8),
        egui::Stroke::new(1.0, border_color),
        egui::StrokeKind::Inside,
    );

    // Soft shadow effect (drawn as a larger, blurred rect behind)
    if response.hovered() {
        let shadow_rect = rect.expand(4.0);
        ui.painter().rect_filled(
            shadow_rect,
            egui::CornerRadius::same((corner_radius + 4.0) as u8),
            AppleColors::SHADOW_HOVER,
        );
    }
}

/// Brand-colored pill badge
pub fn pill_badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) -> egui::Response {
    let text_galley = ui.painter().layout(
        text.to_string(),
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
        color,
        f32::INFINITY,
    );
    let size = egui::vec2(text_galley.size().x + 16.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    let bg = color.gamma_multiply(0.12);
    ui.painter().rect_filled(rect, egui::CornerRadius::same(12), bg);
    ui.painter().galley(
        rect.left_center() + egui::vec2(8.0, -text_galley.size().y * 0.5),
        text_galley,
        color,
    );

    response
}

/// Primary action button with gradient-like solid color
pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .size(14.0)
                .color(AppleColors::SURFACE)
                .strong(),
        )
        .fill(AppleColors::PRIMARY)
        .corner_radius(egui::CornerRadius::same(10))
        .min_size(egui::vec2(100.0, 40.0)),
    )
}

/// Secondary outline button
pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(14.0))
            .fill(AppleColors::SURFACE)
            .stroke(egui::Stroke::new(1.0, AppleColors::BORDER))
            .corner_radius(egui::CornerRadius::same(10))
            .min_size(egui::vec2(80.0, 36.0)),
    )
}

/// Status indicator dot with label
pub fn status_indicator(ui: &mut egui::Ui, ok: bool, label: &str) {
    let color = if ok { AppleColors::SUCCESS } else { AppleColors::DANGER };
    ui.horizontal(|ui| {
        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(dot_rect.center(), 4.0, color);
        ui.label(egui::RichText::new(label).size(12.0).color(AppleColors::TEXT_SECONDARY));
    });
}

/// Page header with title, subtitle and optional right-side action
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(22.0)
                    .strong()
                    .color(AppleColors::TEXT),
            );
            ui.label(
                egui::RichText::new(subtitle)
                    .size(13.0)
                    .color(AppleColors::TEXT_SECONDARY),
            );
        });
    });
    ui.add_space(16.0);
}

/// Page header with a right-aligned button
pub fn page_header_with_action(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    action_label: &str,
    action: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(22.0)
                    .strong()
                    .color(AppleColors::TEXT),
            );
            ui.label(
                egui::RichText::new(subtitle)
                    .size(13.0)
                    .color(AppleColors::TEXT_SECONDARY),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(action_label).clicked() {
                action();
            }
        });
    });
    ui.add_space(16.0);
}

/// Module icon with gradient background circle
pub fn module_icon(ui: &mut egui::Ui, icon: &str, color: egui::Color32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let bg = color.gamma_multiply(0.15);
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same((size * 0.3) as u8),
        bg,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::new(size * 0.5, egui::FontFamily::Proportional),
        color,
    );
    response
}
