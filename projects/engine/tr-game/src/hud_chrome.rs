//! 原版 HUD 铬件：生命心、魔力星、快捷栏底图。
//! 贴图来自正版 `Content/Images` 顶层 `Heart` / `Mana` / `Inventory_Back`。

use std::path::Path;

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};

use crate::xnb::decode_texture_file;

/// 已上传的 HUD 铬件纹理。
#[derive(Debug, Default)]
pub struct HudChrome {
    heart: Option<TextureId>,
    mana: Option<TextureId>,
    inv_back: Option<TextureId>,
    ready: bool,
}

impl HudChrome {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        self.heart = upload_named(draw, assets.hud_heart.as_deref());
        self.mana = upload_named(draw, assets.hud_mana.as_deref());
        self.inv_back = upload_named(draw, assets.hud_inv_back.as_deref());
        tracing::info!(
            heart = self.heart.is_some(),
            mana = self.mana.is_some(),
            inv_back = self.inv_back.is_some(),
            "HUD 铬件已上传"
        );
    }

    /// 绘制一颗生命心。`fill` ∈ [0,1]：按水平裁切表现半心。
    pub fn paint_heart(&self, draw: &mut DrawList, x: f32, y: f32, size: f32, fill: f32) {
        let dest = Rect::new(x, y, size, size);
        if let Some(tex) = self.heart {
            // 空心底：略暗。
            draw.tex_rect(
                tex,
                dest,
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Color::rgba(0.35, 0.12, 0.14, 0.85),
            );
            let f = fill.clamp(0.0, 1.0);
            if f > 0.02 {
                draw.tex_rect(
                    tex,
                    Rect::new(x, y, size * f, size),
                    Rect::new(0.0, 0.0, f, 1.0),
                    Color::rgba(1.0, 1.0, 1.0, 1.0),
                );
            }
        } else {
            paint_heart_fallback(draw, x, y, fill);
        }
    }

    /// 绘制一颗魔力星。
    pub fn paint_mana(&self, draw: &mut DrawList, x: f32, y: f32, size: f32, fill: f32) {
        let dest = Rect::new(x, y, size, size);
        if let Some(tex) = self.mana {
            draw.tex_rect(
                tex,
                dest,
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Color::rgba(0.2, 0.25, 0.45, 0.8),
            );
            let f = fill.clamp(0.0, 1.0);
            if f > 0.02 {
                let a = 0.35 + 0.65 * f;
                draw.tex_rect(
                    tex,
                    dest,
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                    Color::rgba(1.0, 1.0, 1.0, a),
                );
            }
        } else {
            paint_star_fallback(draw, x, y, fill);
        }
    }

    /// 快捷栏槽底图。无色块描边表示选中。
    pub fn paint_slot(
        &self,
        draw: &mut DrawList,
        x: f32,
        y: f32,
        size: f32,
        selected: bool,
    ) {
        if let Some(tex) = self.inv_back {
            let tint = if selected {
                Color::rgba(1.0, 1.0, 0.85, 1.0)
            } else {
                Color::rgba(0.85, 0.88, 0.95, 0.92)
            };
            draw.tex_rect(tex, Rect::new(x, y, size, size), Rect::new(0.0, 0.0, 1.0, 1.0), tint);
            if selected {
                draw.fill_rect(
                    Rect::new(x - 2.0, y - 2.0, size + 4.0, 2.0),
                    Color::rgba(1.0, 0.92, 0.35, 0.95),
                );
                draw.fill_rect(
                    Rect::new(x - 2.0, y + size, size + 4.0, 2.0),
                    Color::rgba(1.0, 0.92, 0.35, 0.95),
                );
            }
        } else {
            draw.fill_rect(
                Rect::new(x - 2.0, y - 2.0, size + 4.0, size + 4.0),
                if selected {
                    crate::palette::HUD_HOTBAR_RING
                } else {
                    crate::palette::HUD_HOTBAR_RING_IDLE
                },
            );
            draw.fill_rect(
                Rect::new(x, y, size, size),
                if selected {
                    crate::palette::HUD_HOTBAR_SELECTED
                } else {
                    crate::palette::HUD_HOTBAR_IDLE
                },
            );
        }
    }
}

fn upload_named(draw: &mut DrawList, path: Option<&Path>) -> Option<TextureId> {
    let path = path?;
    let tex = match decode_texture_file(path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(path = %path.display(), %e, "HUD 铬件解码失败");
            return None;
        }
    };
    match draw.create_texture(tex.width, tex.height, tex.rgba) {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!(?e, name = %tex.name, "HUD 铬件上传失败");
            None
        }
    }
}

fn paint_heart_fallback(draw: &mut DrawList, x: f32, y: f32, fill: f32) {
    let empty = Color::rgba(0.25, 0.08, 0.10, 0.85);
    let full = Color::rgb(0.92, 0.18, 0.28);
    draw.fill_rect(Rect::new(x + 4.0, y + 2.0, 6.0, 6.0), empty);
    draw.fill_rect(Rect::new(x + 12.0, y + 2.0, 6.0, 6.0), empty);
    draw.fill_rect(Rect::new(x + 2.0, y + 6.0, 18.0, 10.0), empty);
    draw.fill_rect(Rect::new(x + 6.0, y + 14.0, 10.0, 6.0), empty);
    if fill > 0.02 {
        let c = if fill >= 0.99 {
            full
        } else {
            Color::rgba(full.r, full.g, full.b, 0.35 + 0.65 * fill)
        };
        let w = 18.0 * fill.clamp(0.0, 1.0);
        draw.fill_rect(Rect::new(x + 2.0, y + 6.0, w, 10.0), c);
        if fill > 0.45 {
            draw.fill_rect(Rect::new(x + 4.0, y + 2.0, 6.0, 6.0), c);
        }
        if fill > 0.7 {
            draw.fill_rect(Rect::new(x + 12.0, y + 2.0, 6.0, 6.0), c);
        }
    }
}

fn fill_disc(draw: &mut DrawList, cx: f32, cy: f32, radius: f32, color: Color) {
    let r = radius.max(1.0);
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    for y in y0..=y1 {
        let dy = y as f32 + 0.5 - cy;
        let inner = r * r - dy * dy;
        if inner <= 0.0 {
            continue;
        }
        let half = inner.sqrt();
        draw.fill_rect(Rect::new(cx - half, y as f32, half * 2.0, 1.0), color);
    }
}

fn paint_star_fallback(draw: &mut DrawList, x: f32, y: f32, fill: f32) {
    let empty = Color::rgba(0.12, 0.16, 0.35, 0.8);
    let full = Color::rgb(0.45, 0.65, 1.0);
    fill_disc(draw, x + 10.0, y + 10.0, 9.0, empty);
    if fill > 0.02 {
        let r = 3.0 + 6.0 * fill.clamp(0.0, 1.0);
        fill_disc(draw, x + 10.0, y + 10.0, r, full);
    }
}
