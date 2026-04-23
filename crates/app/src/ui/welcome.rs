use eframe::egui;
use crate::app::{RustToolsApp, Route};
use crate::theme::AppleColors;

/// 独立欢迎页面 - 应用启动后的首屏
/// 全屏居中展示品牌信息，下方为各模块入口卡片
pub fn show(ui: &mut egui::Ui, app: &mut RustToolsApp) {
    let available = ui.available_size();

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        // ── 品牌区 ──
        ui.add_space(available.y * 0.18);

        // Logo
        let logo_size = 72.0;
        let (logo_rect, _) = ui.allocate_exact_size(egui::vec2(logo_size, logo_size), egui::Sense::hover());
        {
            let painter = ui.painter();
            painter.rect_filled(logo_rect, egui::CornerRadius::same(18), AppleColors::PRIMARY);
            let inner = logo_rect.shrink(logo_size * 0.3);
            painter.rect_filled(inner, egui::CornerRadius::same(6), AppleColors::SURFACE);
        }
        ui.add_space(18.0);

        // 主标题
        ui.label(
            egui::RichText::new("RustTools")
                .size(36.0)
                .strong()
                .color(AppleColors::TEXT),
        );
        ui.add_space(8.0);

        // 副标题
        ui.label(
            egui::RichText::new("一站式高性能工具")
                .size(16.0)
                .color(AppleColors::TEXT_SECONDARY),
        );

        // 分隔线
        ui.add_space(12.0);
        let line_w = 120.0_f32.min(available.x * 0.3);
        let line_rect = ui.allocate_exact_size(egui::vec2(line_w, 2.0), egui::Sense::hover()).1.rect;
        ui.painter().rect_filled(
            line_rect,
            egui::CornerRadius::same(1),
            AppleColors::PRIMARY.gamma_multiply(0.5),
        );

        ui.add_space(available.y * 0.12);

        // ── 模块入口网格 ──
        let gap = 20.0;
        let cols = 3.0_f32;
        let card_w = ((available.x - gap * (cols - 1.0)) / cols).min(280.0);
        let card_h = 140.0;

        // 第一行：YOLO 相关模块
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space((available.x - card_w * cols - gap * (cols - 1.0)).max(0.0) * 0.5);

            welcome_module_card(
                ui, card_w, card_h,
                "YOLO 视觉",
                "目标检测 / 标注 / 训练 / 推理",
                AppleColors::PINK,
                |painter, rect, color| {
                    // 绘制视频/相机图标
                    let body = rect.shrink(4.0);
                    painter.rect_stroke(body, egui::CornerRadius::same(3), egui::Stroke::new(1.5, color), egui::StrokeKind::Inside);
                    let lens = body.center();
                    painter.circle_stroke(lens, body.width().min(body.height()) * 0.25, egui::Stroke::new(1.5, color));
                },
                || { app.route = Route::Hub; },
            );

            welcome_module_card(
                ui, card_w, card_h,
                "HTTP 工具",
                "API 调试 / 请求测试（即将上线）",
                AppleColors::PRIMARY,
                |painter, rect, color| {
                    // 绘制网络/连接图标
                    let c = rect.center();
                    let r = rect.width().min(rect.height()) * 0.25;
                    painter.circle_stroke(c, r, egui::Stroke::new(1.5, color));
                    // 左右连接点
                    painter.circle_filled(c + egui::vec2(-r * 1.4, 0.0), 3.0, color);
                    painter.circle_filled(c + egui::vec2(r * 1.4, 0.0), 3.0, color);
                    painter.line_segment([c + egui::vec2(-r, 0.0), c + egui::vec2(-r * 1.3, 0.0)], egui::Stroke::new(1.5, color));
                    painter.line_segment([c + egui::vec2(r, 0.0), c + egui::vec2(r * 1.3, 0.0)], egui::Stroke::new(1.5, color));
                },
                || {},
            );

            welcome_module_card(
                ui, card_w, card_h,
                "更多工具",
                "持续扩展中...",
                AppleColors::TEXT_TERTIARY,
                |painter, rect, color| {
                    // 三个小圆点
                    let c = rect.center();
                    let spacing = 8.0;
                    painter.circle_filled(c + egui::vec2(-spacing, 0.0), 3.0, color);
                    painter.circle_filled(c, 3.0, color);
                    painter.circle_filled(c + egui::vec2(spacing, 0.0), 3.0, color);
                },
                || {},
            );
        });

        ui.add_space(available.y * 0.15);

        // ── 底部版本信息 ──
        ui.label(
            egui::RichText::new(format!("RustTools v{}", env!("CARGO_PKG_VERSION")))
                .size(11.0)
                .color(AppleColors::TEXT_TERTIARY),
        );
    });
}

fn welcome_module_card(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    title: &str,
    desc: &str,
    accent: egui::Color32,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
    on_click: impl FnOnce(),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();

    // 阴影
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

    // 背景
    painter.rect_filled(rect, egui::CornerRadius::same(14), AppleColors::SURFACE);

    // 顶部彩色条
    let top_bar = egui::Rect::from_min_size(
        rect.min + egui::vec2(16.0, 0.0),
        egui::vec2(rect.width() - 32.0, 3.0),
    );
    painter.rect_filled(top_bar, egui::CornerRadius::same(2), accent);

    // 边框
    let border_color = if hovered { accent } else { AppleColors::BORDER };
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(14),
        egui::Stroke::new(1.0, border_color),
        egui::StrokeKind::Inside,
    );

    // 内容
    let content_rect = rect.shrink(20.0);

    // 图标圆形
    let icon_size = 40.0;
    let icon_rect = egui::Rect::from_center_size(
        content_rect.min + egui::vec2(icon_size * 0.5, content_rect.height() * 0.35),
        egui::vec2(icon_size, icon_size),
    );
    let icon_bg = if hovered { accent } else { accent.gamma_multiply(0.1) };
    painter.circle_filled(icon_rect.center(), icon_size * 0.5, icon_bg);

    // 绘制自定义图标
    let icon_color = if hovered { AppleColors::SURFACE } else { accent };
    draw_icon(painter, icon_rect.shrink(10.0), icon_color);

    // 标题
    painter.text(
        content_rect.min + egui::vec2(icon_size + 16.0, content_rect.height() * 0.25),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(17.0, egui::FontFamily::Proportional),
        AppleColors::TEXT,
    );

    // 描述
    painter.text(
        content_rect.min + egui::vec2(icon_size + 16.0, content_rect.height() * 0.55),
        egui::Align2::LEFT_CENTER,
        desc,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
        AppleColors::TEXT_SECONDARY,
    );

    // 箭头
    let arrow_color = if hovered { accent } else { AppleColors::TEXT_TERTIARY };
    painter.text(
        egui::pos2(rect.max.x - 20.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        ">",
        egui::FontId::new(18.0, egui::FontFamily::Proportional),
        arrow_color,
    );

    if response.clicked() {
        on_click();
    }

    response
}
