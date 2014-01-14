//! 战斗飘字与短时粒子特效。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use spark_widget::label;

#[derive(Debug, Clone)]
pub struct DamageFloater {
    pub x: f32,
    pub y: f32,
    pub amount: f32,
    pub ttl: f32,
    pub max_ttl: f32,
    pub vy: f32,
}

impl DamageFloater {
    pub fn hit(x: f32, y: f32, amount: f32) -> Self {
        Self {
            x,
            y,
            amount: amount.max(1.0),
            ttl: 0.85,
            max_ttl: 0.85,
            vy: -48.0,
        }
    }
}

pub fn push_hit(out: &mut Vec<DamageFloater>, x: f32, y: f32, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    // 轻微错开，避免同帧叠死。
    let nudge = (out.len() as f32) * 6.0;
    out.push(DamageFloater::hit(x + nudge, y, amount));
}

pub fn tick_floaters(list: &mut Vec<DamageFloater>, dt: f32) {
    for f in list.iter_mut() {
        f.ttl -= dt;
        f.y += f.vy * dt;
        f.vy *= 0.92;
    }
    list.retain(|f| f.ttl > 0.0);
}

pub fn draw_floaters(list: &[DamageFloater], draw: &mut DrawList, cam_x: f32, cam_y: f32) {
    for f in list {
        let t = (f.ttl / f.max_ttl).clamp(0.0, 1.0);
        let a = (t * 1.15).min(1.0);
        let sx = f.x - cam_x;
        let sy = f.y - cam_y;
        let text = format!("{:.0}", f.amount);
        label(
            draw,
            sx,
            sy,
            16.0 + (1.0 - t) * 4.0,
            Color::rgba(1.0, 0.92, 0.35, a),
            &text,
        );
    }
}

/// 落地 / 挖掘尘粒。
#[derive(Debug, Clone)]
pub struct DustParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub ttl: f32,
    pub max_ttl: f32,
    pub size: f32,
    /// 0 尘土，1 碎屑，2 凝胶。
    pub tint: u8,
}

/// 脚底扬尘：`impact` 0..=1 控制数量与初速。
pub fn burst_dust(out: &mut Vec<DustParticle>, x: f32, y: f32, impact: f32) {
    let impact = impact.clamp(0.15, 1.0);
    let n = (4.0 + impact * 6.0) as i32;
    for i in 0..n {
        let t = i as f32 / n.max(1) as f32;
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let speed = 28.0 + impact * 55.0 + t * 20.0;
        out.push(DustParticle {
            x: x + side * (2.0 + t * 6.0),
            y: y - 1.0,
            vx: side * speed * (0.55 + t * 0.45),
            vy: -18.0 - impact * 35.0 * (1.0 - t),
            ttl: 0.28 + impact * 0.22,
            max_ttl: 0.28 + impact * 0.22,
            size: 2.0 + impact * 2.5 * (1.0 - t * 0.4),
            tint: 0,
        });
    }
}

/// 挖掘碎屑（偏暖色）。
pub fn burst_chips(out: &mut Vec<DustParticle>, x: f32, y: f32) {
    for i in 0..5 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let t = i as f32 * 0.18;
        out.push(DustParticle {
            x: x + side * 3.0,
            y,
            vx: side * (40.0 + t * 30.0),
            vy: -40.0 - t * 25.0,
            ttl: 0.35,
            max_ttl: 0.35,
            size: 2.5,
            tint: 1,
        });
    }
}

/// 凝胶碎裂：紫色溅点。
pub fn burst_gel(out: &mut Vec<DustParticle>, x: f32, y: f32) {
    for i in 0..7 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let t = i as f32 * 0.13;
        out.push(DustParticle {
            x: x + side * (1.0 + t * 8.0),
            y: y - t * 4.0,
            vx: side * (36.0 + t * 40.0),
            vy: -28.0 - t * 30.0,
            ttl: 0.42,
            max_ttl: 0.42,
            size: 3.0 + t,
            tint: 2,
        });
    }
}

pub fn tick_dust(list: &mut Vec<DustParticle>, dt: f32) {
    for p in list.iter_mut() {
        p.ttl -= dt;
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.vy += 180.0 * dt;
        p.vx *= 0.92;
    }
    list.retain(|p| p.ttl > 0.0);
}

pub fn draw_dust(list: &[DustParticle], draw: &mut DrawList, cam_x: f32, cam_y: f32) {
    for p in list {
        let t = (p.ttl / p.max_ttl).clamp(0.0, 1.0);
        let a = t * 0.55;
        let sx = p.x - cam_x;
        let sy = p.y - cam_y;
        let c = match p.tint {
            1 => Color::rgba(0.72, 0.48, 0.28, a),
            2 => Color::rgba(0.72, 0.32, 0.92, a * 1.15),
            _ => Color::rgba(0.42, 0.38, 0.32, a),
        };
        let s = p.size * (0.65 + 0.35 * t);
        draw.fill_rect(Rect::new(sx - s * 0.5, sy - s * 0.5, s, s), c);
    }
}
