//! HUD 物品图标。能对上正版 `Item_N.xnb` 的用那张图，其余仍是程序化色块。
//! 文件名编号不是夹具 `ItemId`。

use std::collections::HashMap;
use std::path::Path;

use spark_core::{Color, Rect};
use spark_image::PixelImage;
use spark_renderer::{DrawList, TextureId};
use tr_core::ItemId;

const FALLBACK_CELL: u32 = 32;
const FULL_UV: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

/// 单个图标采样视图。
#[derive(Debug, Clone, Copy)]
pub struct IconView {
    pub tex: TextureId,
    pub uv: Rect,
}

/// 按物品索引的原尺寸图标纹理。
pub struct IconAtlas {
    icons: HashMap<u32, TextureId>,
    icon_uv: HashMap<u32, Rect>,
    ready: bool,
}

impl IconAtlas {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            icon_uv: HashMap::new(),
            ready: false,
        }
    }

    /// 首帧上传。优先内容包 PNG（原尺寸），否则程序化占位。
    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        let mut from_sheet = 0u32;
        for id in ItemId::ALL.iter().copied() {
            if let Some((tex, uv)) = sheet_icon(draw, assets, id) {
                self.icons.insert(id.0, tex);
                self.icon_uv.insert(id.0, uv);
                from_sheet += 1;
                continue;
            }
            let tex = assets
                .item_icon
                .get(&id)
                .and_then(|p| upload_png(draw, p))
                .or_else(|| upload_fallback(draw, id));
            if let Some(tex) = tex {
                self.icons.insert(id.0, tex);
            }
        }
        tracing::info!(n = self.icons.len(), from_sheet, "物品图标已上传");
    }

    pub fn view(&self, item: ItemId) -> Option<IconView> {
        let tex = *self.icons.get(&item.0)?;
        let uv = self.icon_uv.get(&item.0).copied().unwrap_or(FULL_UV);
        Some(IconView { tex, uv })
    }
}

fn sheet_icon(
    draw: &mut DrawList,
    assets: &crate::content_boot::ContentAssets,
    id: ItemId,
) -> Option<(TextureId, Rect)> {
    let file_id = crate::sheets::item_file(id)?;
    let path = assets.item_sheets.get(&file_id)?;
    let tex = crate::xnb::decode_texture_file(path).ok()?;
    let image = PixelImage::from_rgba8(tex.width, tex.height, tex.rgba).ok()?;
    let uv = icon_frame_uv(&image);
    let gpu = upload_rgba(draw, image.width(), image.height(), image.into_rgba())?;
    Some((gpu, uv))
}

fn icon_frame_uv(image: &PixelImage) -> Rect {
    let w = image.width();
    let h = image.height();
    if w > 0 && h > w && h.is_multiple_of(w) {
        Rect::new(0.0, 0.0, 1.0, w as f32 / h as f32)
    } else {
        FULL_UV
    }
}

fn upload_rgba(draw: &mut DrawList, w: u32, h: u32, rgba: Vec<u8>) -> Option<TextureId> {
    match draw.create_texture(w, h, rgba) {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!(?e, "图标纹理上传失败");
            None
        }
    }
}

fn upload_png(draw: &mut DrawList, path: &Path) -> Option<TextureId> {
    let (w, h, rgba) = crate::content_boot::ContentAssets::load_rgba(path)?;
    upload_rgba(draw, w, h, rgba)
}

fn upload_fallback(draw: &mut DrawList, id: ItemId) -> Option<TextureId> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    paint_icon(&mut rgba, FALLBACK_CELL, 0, 0, id);
    upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, rgba)
}

fn paint_icon(rgba: &mut [u8], stride_w: u32, col: u32, row: u32, id: ItemId) {
    let x0 = col * FALLBACK_CELL;
    let y0 = row * FALLBACK_CELL;
    let wood = Color::rgb(0.62, 0.42, 0.22);
    let wood_d = Color::rgb(0.42, 0.28, 0.14);
    let metal = Color::rgb(0.72, 0.75, 0.80);
    let metal_d = Color::rgb(0.45, 0.48, 0.52);
    let leaf = Color::rgb(0.35, 0.78, 0.32);
    let fire = Color::rgb(1.0, 0.72, 0.25);
    let magic = Color::rgb(0.55, 0.75, 1.0);
    let purple = Color::rgb(0.72, 0.38, 0.95);

    match id {
        ItemId::DIRT => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                3,
                10,
                10,
                Color::rgb(0.55, 0.38, 0.22),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                3,
                10,
                2,
                Color::rgb(0.40, 0.28, 0.16),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                5,
                7,
                2,
                2,
                Color::rgb(0.35, 0.24, 0.14),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                9,
                9,
                2,
                2,
                Color::rgb(0.68, 0.48, 0.28),
            );
        }
        ItemId::STONE => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                3,
                10,
                10,
                Color::rgb(0.52, 0.55, 0.60),
            );
            fill(rgba, stride_w, x0, y0, 4, 5, 3, 2, metal_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                9,
                8,
                3,
                2,
                Color::rgb(0.65, 0.68, 0.72),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                3,
                10,
                1,
                Color::rgb(0.38, 0.40, 0.44),
            );
        }
        ItemId::SAND => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                4,
                10,
                9,
                Color::rgb(0.86, 0.74, 0.42),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                6,
                2,
                1,
                Color::rgb(0.95, 0.85, 0.55),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                9,
                9,
                2,
                1,
                Color::rgb(0.70, 0.58, 0.30),
            );
        }
        ItemId::SNOW => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                4,
                10,
                9,
                Color::rgb(0.88, 0.92, 0.98),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                5,
                6,
                2,
                1,
                Color::rgb(0.70, 0.78, 0.90),
            );
        }
        ItemId::SCRAP => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                5,
                8,
                6,
                Color::rgb(0.72, 0.55, 0.28),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                5,
                6,
                2,
                2,
                Color::rgb(0.95, 0.82, 0.35),
            );
            fill(rgba, stride_w, x0, y0, 9, 7, 2, 2, metal);
        }
        ItemId::WOOD => {
            fill(rgba, stride_w, x0, y0, 5, 2, 6, 12, wood);
            fill(rgba, stride_w, x0, y0, 5, 2, 1, 12, wood_d);
            fill(rgba, stride_w, x0, y0, 10, 2, 1, 12, wood_d);
            fill(rgba, stride_w, x0, y0, 6, 5, 4, 1, wood_d);
            fill(rgba, stride_w, x0, y0, 6, 9, 4, 1, wood_d);
        }
        ItemId::COPPER_ORE | ItemId::IRON_ORE => {
            let vein = if matches!(id, ItemId::COPPER_ORE) {
                Color::rgb(0.85, 0.55, 0.28)
            } else {
                Color::rgb(0.55, 0.58, 0.62)
            };
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                3,
                10,
                10,
                Color::rgb(0.40, 0.42, 0.46),
            );
            fill(rgba, stride_w, x0, y0, 5, 5, 3, 2, vein);
            fill(rgba, stride_w, x0, y0, 9, 8, 2, 3, vein);
            fill(rgba, stride_w, x0, y0, 6, 10, 2, 2, vein);
        }
        ItemId::COPPER_BAR | ItemId::IRON_BAR => {
            let c = if matches!(id, ItemId::COPPER_BAR) {
                Color::rgb(0.90, 0.58, 0.28)
            } else {
                metal
            };
            fill(rgba, stride_w, x0, y0, 4, 6, 8, 5, c);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                6,
                8,
                1,
                Color::rgb(1.0, 0.95, 0.8),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                10,
                8,
                1,
                Color::rgb(c.r * 0.55, c.g * 0.55, c.b * 0.55),
            );
        }
        ItemId::WORKBENCH => {
            fill(rgba, stride_w, x0, y0, 2, 7, 12, 6, wood);
            fill(rgba, stride_w, x0, y0, 3, 5, 2, 2, wood_d);
            fill(rgba, stride_w, x0, y0, 11, 5, 2, 2, wood_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                8,
                3,
                2,
                Color::rgb(0.35, 0.32, 0.28),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                9,
                8,
                3,
                2,
                Color::rgb(0.85, 0.7, 0.35),
            );
        }
        ItemId::FURNACE => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                4,
                10,
                10,
                Color::rgb(0.38, 0.36, 0.34),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                5,
                7,
                6,
                5,
                Color::rgb(0.12, 0.10, 0.10),
            );
            fill(rgba, stride_w, x0, y0, 6, 8, 4, 3, fire);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                9,
                2,
                1,
                Color::rgb(1.0, 0.9, 0.5),
            );
        }
        ItemId::CHEST => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                5,
                10,
                8,
                Color::rgb(0.78, 0.55, 0.25),
            );
            fill(rgba, stride_w, x0, y0, 3, 8, 10, 1, wood_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                8,
                2,
                2,
                Color::rgb(0.95, 0.82, 0.35),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                5,
                10,
                1,
                Color::rgb(0.55, 0.38, 0.16),
            );
        }
        ItemId::BED => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                2,
                8,
                12,
                5,
                Color::rgb(0.75, 0.32, 0.38),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                2,
                7,
                4,
                2,
                Color::rgb(0.92, 0.9, 0.85),
            );
            fill(rgba, stride_w, x0, y0, 2, 12, 12, 1, wood_d);
        }
        ItemId::PLATFORM => {
            fill(rgba, stride_w, x0, y0, 2, 7, 12, 3, wood);
            fill(rgba, stride_w, x0, y0, 2, 7, 12, 1, wood_d);
            fill(rgba, stride_w, x0, y0, 4, 10, 1, 3, wood_d);
            fill(rgba, stride_w, x0, y0, 11, 10, 1, 3, wood_d);
        }
        ItemId::LADDER => {
            fill(rgba, stride_w, x0, y0, 5, 2, 2, 12, wood);
            fill(rgba, stride_w, x0, y0, 9, 2, 2, 12, wood);
            fill(rgba, stride_w, x0, y0, 5, 4, 6, 1, wood_d);
            fill(rgba, stride_w, x0, y0, 5, 8, 6, 1, wood_d);
            fill(rgba, stride_w, x0, y0, 5, 12, 6, 1, wood_d);
        }
        ItemId::SAPLING => {
            fill(rgba, stride_w, x0, y0, 7, 10, 2, 4, wood);
            fill(rgba, stride_w, x0, y0, 5, 4, 6, 6, leaf);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                3,
                4,
                2,
                Color::rgb(0.45, 0.88, 0.40),
            );
            px(rgba, stride_w, x0, y0, 4, 6, leaf);
            px(rgba, stride_w, x0, y0, 11, 7, leaf);
        }
        ItemId::TORCH => {
            fill(rgba, stride_w, x0, y0, 7, 8, 2, 6, wood);
            fill(rgba, stride_w, x0, y0, 6, 3, 4, 5, fire);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                2,
                2,
                2,
                Color::rgb(1.0, 0.95, 0.55),
            );
            px(rgba, stride_w, x0, y0, 5, 5, Color::rgb(1.0, 0.55, 0.15));
            px(rgba, stride_w, x0, y0, 10, 4, Color::rgb(1.0, 0.55, 0.15));
        }
        ItemId::WARP => {
            fill(rgba, stride_w, x0, y0, 5, 3, 6, 10, purple);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                4,
                4,
                8,
                Color::rgb(0.25, 0.12, 0.35),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                6,
                2,
                4,
                Color::rgb(0.9, 0.7, 1.0),
            );
            px(rgba, stride_w, x0, y0, 4, 5, magic);
            px(rgba, stride_w, x0, y0, 11, 9, magic);
        }
        ItemId::GEL => {
            fill(rgba, stride_w, x0, y0, 5, 5, 6, 7, purple);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                4,
                4,
                2,
                Color::rgb(0.9, 0.65, 1.0),
            );
            px(rgba, stride_w, x0, y0, 6, 7, Color::rgb(1.0, 0.95, 1.0));
            px(rgba, stride_w, x0, y0, 9, 7, Color::rgb(1.0, 0.95, 1.0));
        }
        ItemId::WOOD_PICK | ItemId::STONE_PICK | ItemId::COPPER_PICK => {
            let head = match id {
                ItemId::STONE_PICK => metal,
                ItemId::COPPER_PICK => Color::rgb(0.90, 0.58, 0.28),
                _ => wood,
            };
            for i in 0..10 {
                px(rgba, stride_w, x0, y0, 4 + i, 11 - i, wood);
                px(rgba, stride_w, x0, y0, 5 + i, 11 - i, wood_d);
            }
            fill(rgba, stride_w, x0, y0, 9, 2, 5, 3, head);
            fill(rgba, stride_w, x0, y0, 11, 4, 3, 3, head);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                9,
                2,
                5,
                1,
                Color::rgb(1.0, 1.0, 1.0),
            );
        }
        ItemId::WOOD_SWORD => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                2,
                2,
                9,
                Color::rgb(0.85, 0.82, 0.70),
            );
            fill(rgba, stride_w, x0, y0, 6, 10, 4, 2, wood);
            fill(rgba, stride_w, x0, y0, 7, 12, 2, 2, wood_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                2,
                2,
                1,
                Color::rgb(1.0, 0.98, 0.9),
            );
        }
        ItemId::WOOD_BOW => {
            for i in 0..8 {
                px(rgba, stride_w, x0, y0, 4, 3 + i, wood);
                px(rgba, stride_w, x0, y0, 11, 3 + i, wood);
            }
            px(rgba, stride_w, x0, y0, 5, 2, wood);
            px(rgba, stride_w, x0, y0, 10, 2, wood);
            px(rgba, stride_w, x0, y0, 5, 11, wood);
            px(rgba, stride_w, x0, y0, 10, 11, wood);
            for i in 0..6 {
                px(
                    rgba,
                    stride_w,
                    x0,
                    y0,
                    5 + i,
                    3 + i,
                    Color::rgb(0.9, 0.88, 0.8),
                );
            }
        }
        ItemId::WOOD_ARROW => {
            fill(rgba, stride_w, x0, y0, 7, 2, 2, 10, wood);
            fill(rgba, stride_w, x0, y0, 6, 2, 4, 2, metal);
            fill(rgba, stride_w, x0, y0, 6, 12, 4, 2, leaf);
        }
        ItemId::GEL_STAFF => {
            fill(rgba, stride_w, x0, y0, 7, 5, 2, 9, wood);
            fill(rgba, stride_w, x0, y0, 6, 2, 4, 4, magic);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                3,
                2,
                2,
                Color::rgb(0.95, 0.98, 1.0),
            );
            px(rgba, stride_w, x0, y0, 5, 3, purple);
            px(rgba, stride_w, x0, y0, 10, 4, purple);
        }
        ItemId::WOOD_ARMOR => {
            fill(rgba, stride_w, x0, y0, 4, 4, 8, 9, wood);
            fill(rgba, stride_w, x0, y0, 5, 3, 6, 2, wood_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                6,
                4,
                3,
                Color::rgb(0.75, 0.55, 0.32),
            );
            fill(rgba, stride_w, x0, y0, 4, 4, 2, 4, wood_d);
            fill(rgba, stride_w, x0, y0, 10, 4, 2, 4, wood_d);
        }
        ItemId::GRAPPLE => {
            fill(rgba, stride_w, x0, y0, 7, 3, 2, 8, wood);
            fill(rgba, stride_w, x0, y0, 6, 10, 4, 3, metal);
            fill(rgba, stride_w, x0, y0, 5, 11, 2, 2, metal_d);
            fill(rgba, stride_w, x0, y0, 9, 11, 2, 2, metal_d);
            px(rgba, stride_w, x0, y0, 7, 2, Color::rgb(0.9, 0.9, 0.95));
        }
        ItemId::CLOUD_BOTTLE => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                5,
                5,
                6,
                9,
                Color::rgb(0.45, 0.78, 0.92),
            );
            fill(rgba, stride_w, x0, y0, 5, 9, 6, 5, purple);
            fill(rgba, stride_w, x0, y0, 6, 3, 4, 2, metal);
            fill(rgba, stride_w, x0, y0, 6, 1, 4, 2, wood);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                6,
                1,
                3,
                Color::rgb(0.9, 0.96, 1.0),
            );
        }
        ItemId::WOOD_HAMMER => {
            fill(rgba, stride_w, x0, y0, 7, 2, 2, 8, wood);
            fill(rgba, stride_w, x0, y0, 4, 9, 8, 3, metal);
            fill(rgba, stride_w, x0, y0, 4, 9, 8, 1, metal_d);
        }
        ItemId::ROPE => {
            fill(rgba, stride_w, x0, y0, 7, 2, 2, 12, wood);
            for i in 0..4 {
                fill(rgba, stride_w, x0, y0, 6, 3 + i * 3, 4, 1, wood_d);
            }
        }
        ItemId::CLOTH_BAG => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                6,
                8,
                8,
                Color::rgb(0.72, 0.55, 0.32),
            );
            fill(rgba, stride_w, x0, y0, 5, 4, 6, 3, wood_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                3,
                4,
                2,
                Color::rgb(0.85, 0.7, 0.4),
            );
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                7,
                9,
                2,
                2,
                Color::rgb(0.95, 0.82, 0.45),
            );
        }
        ItemId::TRAVEL_PACK => {
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                3,
                5,
                10,
                9,
                Color::rgb(0.42, 0.32, 0.22),
            );
            fill(rgba, stride_w, x0, y0, 4, 3, 8, 3, wood_d);
            fill(rgba, stride_w, x0, y0, 2, 6, 2, 6, metal_d);
            fill(rgba, stride_w, x0, y0, 12, 6, 2, 6, metal_d);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                6,
                8,
                4,
                3,
                Color::rgb(0.55, 0.38, 0.22),
            );
        }
        _ => {
            let c = Color::rgb(0.5, 0.55, 0.6);
            fill(rgba, stride_w, x0, y0, 4, 4, 8, 8, c);
            fill(
                rgba,
                stride_w,
                x0,
                y0,
                4,
                4,
                8,
                1,
                Color::rgb(0.75, 0.78, 0.82),
            );
        }
    }
}

fn px(rgba: &mut [u8], stride_w: u32, x0: u32, y0: u32, x: i32, y: i32, c: Color) {
    if x < 0 || y < 0 || x >= FALLBACK_CELL as i32 || y >= FALLBACK_CELL as i32 {
        return;
    }
    put_px(rgba, stride_w, x0 + x as u32, y0 + y as u32, c);
}

fn fill(
    rgba: &mut [u8],
    stride_w: u32,
    x0: u32,
    y0: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    c: Color,
) {
    for dy in 0..h {
        for dx in 0..w {
            px(rgba, stride_w, x0, y0, x + dx, y + dy, c);
        }
    }
}

fn put_px(rgba: &mut [u8], stride_w: u32, x: u32, y: u32, c: Color) {
    let i = ((y * stride_w + x) * 4) as usize;
    if i + 3 >= rgba.len() {
        return;
    }
    rgba[i] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 3] = (c.a.clamp(0.0, 1.0) * 255.0) as u8;
}
