//! 正版贴图条。启动时解码的图直接画在界面上。泥土方块另用 `Tiles_0` 第一格。

use spark_core::{Color, Rect};
use spark_image::PixelImage;
use spark_renderer::{DrawList, TextureId};
use spark_widget::{label, panel};

use crate::app::TerrariaApp;
use crate::xnb::RgbaTexture;

struct Slot {
    tex: TextureId,
    w: u32,
    h: u32,
    label: String,
}

pub(crate) struct ProofGpu {
    done: bool,
    logged: bool,
    slots: Vec<Slot>,
}

impl ProofGpu {
    pub(crate) fn new() -> Self {
        Self {
            done: false,
            logged: false,
            slots: Vec::new(),
        }
    }

    fn upload(&mut self, draw: &mut DrawList, images: &[RgbaTexture]) {
        self.done = true;
        for image in images {
            let pixels = match PixelImage::from_rgba8(image.width, image.height, image.rgba.clone())
            {
                Ok(pixels) => pixels,
                Err(e) => {
                    tracing::error!(name = %image.name, ?e, "正版贴图无法交给像素图");
                    continue;
                }
            };
            let (w, h) = (pixels.width(), pixels.height());
            match draw.create_texture(w, h, pixels.into_rgba()) {
                Ok(tex) => self.slots.push(Slot {
                    tex,
                    w: image.width,
                    h: image.height,
                    label: image.name.clone(),
                }),
                Err(e) => {
                    tracing::error!(name = %image.name, ?e, "正版证明贴图上传失败");
                }
            }
        }
    }
}

impl TerrariaApp {
    pub(crate) fn paint_boot_frames(&mut self, draw: &mut DrawList) {
        if !self.proof_gpu.done {
            let images = self.content_assets.boot_frames.clone();
            self.proof_gpu.upload(draw, &images);
        }
        if self.proof_gpu.slots.is_empty() {
            return;
        }
        if !self.proof_gpu.logged {
            self.proof_gpu.logged = true;
            tracing::info!(
                target: "tr.content",
                n = self.proof_gpu.slots.len(),
                "正版证明贴图已绘制"
            );
        }

        let box_px = 112.0;
        let gap = 8.0;
        let mut widths = Vec::with_capacity(self.proof_gpu.slots.len());
        for slot in &self.proof_gpu.slots {
            widths.push(fit(slot.w, slot.h, box_px).0);
        }
        let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
        let panel_w = total + 24.0;
        let panel_h = box_px + 46.0;
        let x0 = (self.screen_w - panel_w - 16.0).max(8.0);
        let y0 = 12.0;
        panel(
            draw,
            Rect::new(x0, y0, panel_w, panel_h),
            Color::rgba(0.05, 0.07, 0.12, 0.88),
        );
        label(
            draw,
            x0 + 10.0,
            y0 + 6.0,
            12.0,
            Color::rgb(0.85, 0.9, 0.75),
            "正版贴图",
        );
        let mut x = x0 + 12.0;
        for slot in &self.proof_gpu.slots {
            let (dw, dh) = fit(slot.w, slot.h, box_px);
            let y = y0 + 24.0 + (box_px - dh) * 0.5;
            draw.tex_rect(
                slot.tex,
                Rect::new(x, y, dw, dh),
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
            x += dw + gap;
        }
    }
}

fn fit(w: u32, h: u32, box_px: f32) -> (f32, f32) {
    let w = w.max(1) as f32;
    let h = h.max(1) as f32;
    let s = (box_px / w).min(box_px / h);
    (w * s, h * s)
}
