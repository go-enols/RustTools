//! 模块图标绘制 - 使用 egui painter 绘制精致的几何图标
//! 所有图标在指定大小的矩形区域内居中绘制

use eframe::egui;
use crate::app::Route;
use crate::theme::AppleColors;

/// 绘制导航栏小图标（24x24 区域）
pub fn draw_nav_icon(painter: &egui::Painter, rect: egui::Rect, route: Route, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let stroke = egui::Stroke::new(s * 0.08, color);
    let fill = color;

    match route {
        Route::Welcome => {
            // 星形/钻石
            let r = s * 0.35;
            painter.circle_filled(c, r, fill);
        }
        Route::Hub => {
            // 2x2 网格
            let cell = s * 0.18;
            let gap = s * 0.08;
            let offset = cell + gap;
            for dx in [-1.0, 1.0] {
                for dy in [-1.0, 1.0] {
                    let p = c + egui::vec2(dx * offset * 0.5, dy * offset * 0.5);
                    let r = egui::Rect::from_center_size(p, egui::vec2(cell, cell));
                    painter.rect_filled(r, egui::CornerRadius::same(2), fill);
                }
            }
        }
        Route::Project => {
            // 文件夹：主体 + 标签页
            let w = s * 0.7;
            let h = s * 0.5;
            let body = egui::Rect::from_center_size(c + egui::vec2(0.0, s * 0.05), egui::vec2(w, h));
            painter.rect_filled(body, egui::CornerRadius::same(3), fill);
            // 标签页
            let tab = egui::Rect::from_min_size(
                body.min + egui::vec2(s * 0.08, -s * 0.12),
                egui::vec2(w * 0.35, s * 0.15),
            );
            painter.rect_filled(tab, egui::CornerRadius::same(2), fill);
        }
        Route::Annotation => {
            // 画笔/铅笔：斜线 + 笔尖
            let tip = c + egui::vec2(s * 0.2, -s * 0.2);
            let tail = c + egui::vec2(-s * 0.15, s * 0.15);
            painter.line_segment([tail, tip], stroke);
            // 笔尖高光
            painter.circle_filled(tip, s * 0.06, fill);
        }
        Route::Training => {
            // 向上的折线趋势
            let p1 = c + egui::vec2(-s * 0.25, s * 0.15);
            let p2 = c + egui::vec2(-s * 0.05, s * 0.05);
            let p3 = c + egui::vec2(s * 0.05, -s * 0.05);
            let p4 = c + egui::vec2(s * 0.2, -s * 0.2);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            // 终点圆点
            painter.circle_filled(p4, s * 0.06, fill);
        }
        Route::Video => {
            // 播放三角形
            let left = c + egui::vec2(-s * 0.12, -s * 0.18);
            let top = c + egui::vec2(s * 0.15, 0.0);
            let bottom = c + egui::vec2(-s * 0.12, s * 0.18);
            painter.line_segment([left, top], stroke);
            painter.line_segment([top, bottom], stroke);
            painter.line_segment([bottom, left], stroke);
        }
        Route::Desktop => {
            // 显示器：矩形 + 底座
            let screen = egui::Rect::from_center_size(
                c + egui::vec2(0.0, -s * 0.05),
                egui::vec2(s * 0.6, s * 0.4),
            );
            painter.rect_stroke(screen, egui::CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            // 底座
            let stand = egui::Rect::from_center_size(
                c + egui::vec2(0.0, s * 0.22),
                egui::vec2(s * 0.2, s * 0.04),
            );
            painter.rect_filled(stand, egui::CornerRadius::same(1), fill);
        }
        Route::Device => {
            // 芯片：外框 + 内圆
            let outer = egui::Rect::from_center_size(c, egui::vec2(s * 0.55, s * 0.55));
            painter.rect_stroke(outer, egui::CornerRadius::same(3), stroke, egui::StrokeKind::Inside);
            painter.circle_filled(c, s * 0.12, fill);
        }
        Route::Settings => {
            // 齿轮简化：圆 + 十字
            let r = s * 0.18;
            painter.circle_stroke(c, r, stroke);
            let cross = s * 0.1;
            painter.line_segment(
                [c + egui::vec2(-cross, 0.0), c + egui::vec2(cross, 0.0)],
                stroke,
            );
            painter.line_segment(
                [c + egui::vec2(0.0, -cross), c + egui::vec2(0.0, cross)],
                stroke,
            );
        }
    }
}

/// 绘制空状态图标（柔和线框风格）
pub fn draw_empty_folder(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(2.0, color);
    let body = rect.shrink(4.0);
    painter.rect_stroke(body, egui::CornerRadius::same(8), stroke, egui::StrokeKind::Inside);
    let tab = egui::Rect::from_min_size(
        body.min + egui::vec2(8.0, -6.0),
        egui::vec2(24.0, 12.0),
    );
    painter.rect_stroke(tab, egui::CornerRadius::same(4), stroke, egui::StrokeKind::Inside);
}

pub fn draw_empty_image(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(2.0, color);
    let img_rect = rect.shrink(6.0);
    painter.rect_stroke(img_rect, egui::CornerRadius::same(6), stroke, egui::StrokeKind::Inside);
    let m1 = img_rect.min + egui::vec2(img_rect.width() * 0.25, img_rect.height() * 0.65);
    let m2 = img_rect.min + egui::vec2(img_rect.width() * 0.5, img_rect.height() * 0.35);
    let m3 = img_rect.min + egui::vec2(img_rect.width() * 0.75, img_rect.height() * 0.65);
    painter.line_segment([m1, m2], stroke);
    painter.line_segment([m2, m3], stroke);
    let sun_center = img_rect.min + egui::vec2(img_rect.width() * 0.7, img_rect.height() * 0.25);
    painter.circle_stroke(sun_center, 5.0, stroke);
}
