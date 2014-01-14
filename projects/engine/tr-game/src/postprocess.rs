//! 屏幕后期叠层：色分级、洞穴暗角、受击闪白。
//!
//! 不改世界绘制权威，只在最终帧上叠加氛围。

use spark_core::Color;
use spark_renderer::DrawList;

/// 昼夜色分级：白天略暖、夜晚略冷蓝，强度刻意克制。
pub fn paint_color_grade(draw: &mut DrawList, sw: f32, sh: f32, dayness: f32) {
    let night = (1.0 - dayness).clamp(0.0, 1.0);
    if night > 0.08 {
        let a = 0.06 + 0.10 * night;
        draw.fill_rect(
            spark_core::Rect::new(0.0, 0.0, sw, sh),
            Color::rgba(0.08, 0.12, 0.28, a),
        );
    }
    if dayness > 0.55 {
        let warm = ((dayness - 0.55) / 0.45).clamp(0.0, 1.0);
        // 黄昏暖边：只压画面上下沿，避免整屏发橘。
        let a = 0.04 * warm;
        let band = sh * 0.18;
        draw.fill_rect(
            spark_core::Rect::new(0.0, 0.0, sw, band),
            Color::rgba(0.55, 0.28, 0.22, a),
        );
        draw.fill_rect(
            spark_core::Rect::new(0.0, sh - band, sw, band),
            Color::rgba(0.45, 0.22, 0.18, a * 0.85),
        );
    }
}

/// 洞穴边缘暗角：`cave` 0..=1。
pub fn paint_cave_vignette(draw: &mut DrawList, sw: f32, sh: f32, cave: f32) {
    if cave <= 0.05 {
        return;
    }
    let a = 0.22 * cave;
    let band = 72.0;
    draw.fill_rect(
        spark_core::Rect::new(0.0, 0.0, sw, band),
        Color::rgba(0.0, 0.0, 0.02, a),
    );
    draw.fill_rect(
        spark_core::Rect::new(0.0, sh - band, sw, band),
        Color::rgba(0.0, 0.0, 0.02, a),
    );
    draw.fill_rect(
        spark_core::Rect::new(0.0, 0.0, band, sh),
        Color::rgba(0.0, 0.0, 0.02, a * 0.85),
    );
    draw.fill_rect(
        spark_core::Rect::new(sw - band, 0.0, band, sh),
        Color::rgba(0.0, 0.0, 0.02, a * 0.85),
    );
}

/// 受击闪屏：`t` 为剩余无敌帧归一化强度（刚受伤时接近 1）。
pub fn paint_hurt_flash(draw: &mut DrawList, sw: f32, sh: f32, t: f32) {
    let t = t.clamp(0.0, 1.0);
    if t < 0.05 {
        return;
    }
    let a = 0.22 * t;
    let band = 48.0 + 36.0 * t;
    draw.fill_rect(
        spark_core::Rect::new(0.0, 0.0, sw, band),
        Color::rgba(0.85, 0.12, 0.18, a),
    );
    draw.fill_rect(
        spark_core::Rect::new(0.0, sh - band, sw, band),
        Color::rgba(0.85, 0.12, 0.18, a),
    );
    draw.fill_rect(
        spark_core::Rect::new(0.0, 0.0, band * 0.7, sh),
        Color::rgba(0.75, 0.08, 0.14, a * 0.7),
    );
    draw.fill_rect(
        spark_core::Rect::new(sw - band * 0.7, 0.0, band * 0.7, sh),
        Color::rgba(0.75, 0.08, 0.14, a * 0.7),
    );
}
