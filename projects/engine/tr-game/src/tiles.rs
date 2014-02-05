//! 方块纹理。图集编号来自内容目录，不按类型常量取号。
//! 地形块按邻接 framing 从 `Tiles_N` 采样 16 像素格（步长 18）。

use std::collections::HashMap;
use std::path::Path;

use spark_core::{Color, Rect};
use spark_image::PixelImage;
use spark_renderer::{DrawList, TextureId};
use tr_core::{BlockId, WallId};

use crate::tile_frame::{self, CELL};
use crate::world::World;

/// 仅无 PNG 时的程序化兜底边长。
const FALLBACK_CELL: u32 = 32;

const FULL_UV: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

/// 单个瓦片采样视图：独立纹理 + UV（通常为整张 `[0,1]²`）。
#[derive(Debug, Clone, Copy)]
pub struct TileView {
    pub tex: TextureId,
    pub uv: Rect,
}

#[derive(Debug, Clone, Copy)]
struct SheetMeta {
    tex: TextureId,
    w: u32,
    h: u32,
    /// 无法 framing 时的兜底 UV（满连接中心格）。
    fallback_uv: Rect,
}

/// 按方块 / 墙 / 特效用途索引的原尺寸纹理集。
pub struct TileAtlas {
    blocks: HashMap<u32, TextureId>,
    /// Content 图集元数据（可 framing）。
    sheets: HashMap<u32, SheetMeta>,
    /// 不为整张 `[0,1]²` 的采样，例如家具第一格。
    block_uv: HashMap<u32, Rect>,
    walls: HashMap<u8, TextureId>,
    /// Content 墙图集（可 framing）。
    wall_sheets: HashMap<u8, SheetMeta>,
    halo: Option<TextureId>,
    white: Option<TextureId>,
    crack: Option<TextureId>,
    slime: Option<TextureId>,
    slime_uv: Rect,
    ready: bool,
}

impl TileAtlas {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            sheets: HashMap::new(),
            block_uv: HashMap::new(),
            walls: HashMap::new(),
            wall_sheets: HashMap::new(),
            halo: None,
            white: None,
            crack: None,
            slime: None,
            slime_uv: FULL_UV,
            ready: false,
        }
    }

    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        let sheets = self.upload_sheets(draw, assets);
        let wall_n = self.upload_wall_sheets(draw, assets);

        for id in [
            BlockId::DIRT,
            BlockId::GRASS,
            BlockId::STONE,
            BlockId::WOOD,
            BlockId::TREES,
            BlockId::LEAF,
            BlockId::WORKBENCH,
            BlockId::SAPLING,
            BlockId::TORCH,
            BlockId::PLATFORM,
            BlockId::CHEST,
            BlockId::LADDER,
            BlockId::ROPE,
            BlockId::SAND,
            BlockId::SNOW,
            BlockId::COPPER_ORE,
            BlockId::IRON_ORE,
            BlockId::FURNACE,
            BlockId::BED,
            BlockId::WATER,
        ] {
            let tex = if self.blocks.contains_key(&id.0) {
                None
            } else {
                assets
                    .block_side
                    .get(&id)
                    .and_then(|p| upload_png(draw, p))
                    .or_else(|| upload_fallback_block(draw, id))
            };
            if let Some(tex) = tex {
                self.blocks.insert(id.0, tex);
            }
        }

        for (wall, base) in [
            (WallId::DIRT, Color::rgb(0.30, 0.20, 0.14)),
            (WallId::STONE, Color::rgb(0.24, 0.26, 0.30)),
            (WallId::WOOD, Color::rgb(0.38, 0.26, 0.14)),
        ] {
            if self.wall_sheets.contains_key(&wall.0) || self.walls.contains_key(&wall.0) {
                continue;
            }
            let tex = assets
                .wall_tex
                .get(&wall)
                .and_then(|p| upload_png(draw, p))
                .or_else(|| upload_fallback_wall(draw, base));
            if let Some(tex) = tex {
                self.walls.insert(wall.0, tex);
            }
        }

        self.halo = upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, paint_halo_rgba());
        self.crack = upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, paint_cracks_rgba());
        self.white = upload_rgba(
            draw,
            FALLBACK_CELL,
            FALLBACK_CELL,
            solid_cell(Color::rgb(1.0, 1.0, 1.0)),
        );
        self.slime = self
            .slime
            .or_else(|| upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, paint_slime_rgba()));

        tracing::info!(
            blocks = self.blocks.len(),
            sheets,
            framed = self.sheets.len(),
            walls = self.walls.len() + self.wall_sheets.len(),
            wall_sheets = wall_n,
            "瓦片纹理已上传"
        );
    }

    fn upload_sheets(
        &mut self,
        draw: &mut DrawList,
        assets: &crate::content_boot::ContentAssets,
    ) -> u32 {
        let mut n = 0u32;
        let Some(content) = tr_core::try_content() else {
            return n;
        };
        for (id, def) in content.iter_blocks() {
            let Some(file_id) = def.texture_file else {
                continue;
            };
            let Some(path) = assets.tile_sheets.get(&file_id) else {
                continue;
            };
            let Ok(tex) = crate::xnb::decode_texture_file(path) else {
                tracing::warn!(file = file_id, "Content 方块图集解码失败");
                continue;
            };
            let Ok(image) = PixelImage::from_rgba8(tex.width, tex.height, tex.rgba) else {
                continue;
            };
            let w = image.width();
            let h = image.height();
            let framed = def.framed_terrain && w >= CELL && h >= CELL;
            let fallback_uv = if framed {
                tile_frame::frame_to_uv(18, 18, w, h).unwrap_or(FULL_UV)
            } else {
                let Some(uv) = best_cell_uv(&image, id == BlockId::GRASS) else {
                    tracing::info!(
                        file = file_id,
                        w,
                        h,
                        "方块图集切不出第一格，这块仍用程序化色块"
                    );
                    continue;
                };
                uv
            };
            let Some(gpu) = upload_rgba(draw, w, h, image.into_rgba()) else {
                continue;
            };
            self.blocks.insert(id.0, gpu);
            if framed {
                self.sheets.insert(
                    id.0,
                    SheetMeta {
                        tex: gpu,
                        w,
                        h,
                        fallback_uv,
                    },
                );
            } else {
                self.block_uv.insert(id.0, fallback_uv);
            }
            n += 1;
        }
        if let Some(path) = assets.npc_sheets.get(&crate::sheets::SLIME_NPC_FILE) {
            if let Some((gpu, uv)) = upload_slime_frame(draw, path) {
                self.slime = Some(gpu);
                self.slime_uv = uv;
            }
        }
        n
    }

    fn upload_wall_sheets(
        &mut self,
        draw: &mut DrawList,
        assets: &crate::content_boot::ContentAssets,
    ) -> u32 {
        let mut n = 0u32;
        for id in [WallId::DIRT, WallId::STONE, WallId::WOOD] {
            let Some(file_id) = crate::sheets::wall_file(id) else {
                continue;
            };
            let Some(path) = assets.wall_sheets.get(&file_id) else {
                continue;
            };
            let Ok(tex) = crate::xnb::decode_texture_file(path) else {
                tracing::warn!(file = file_id, "Content 墙图集解码失败");
                continue;
            };
            let Ok(image) = PixelImage::from_rgba8(tex.width, tex.height, tex.rgba) else {
                continue;
            };
            let w = image.width();
            let h = image.height();
            if w < CELL || h < CELL {
                continue;
            }
            let fallback_uv = tile_frame::frame_to_uv(18, 18, w, h).unwrap_or(FULL_UV);
            let Some(gpu) = upload_rgba(draw, w, h, image.into_rgba()) else {
                continue;
            };
            self.walls.insert(id.0, gpu);
            self.wall_sheets.insert(
                id.0,
                SheetMeta {
                    tex: gpu,
                    w,
                    h,
                    fallback_uv,
                },
            );
            n += 1;
        }
        n
    }

    /// 静态采样（家具等）。地形请用 [`Self::block_framed`]。
    pub fn block(&self, id: BlockId) -> Option<TileView> {
        if let Some(meta) = self.sheets.get(&id.0) {
            return Some(TileView {
                tex: meta.tex,
                uv: meta.fallback_uv,
            });
        }
        let tex = *self.blocks.get(&id.0)?;
        let uv = self.block_uv.get(&id.0).copied().unwrap_or(FULL_UV);
        Some(TileView { tex, uv })
    }

    /// 按已写入的帧采样。没有帧时不改用图集上的某一格。
    pub fn block_framed(&self, world: &World, tx: i32, ty: i32) -> Option<TileView> {
        let id = world.get(tx, ty);
        if let Some(meta) = self.sheets.get(&id.0) {
            let (fx, fy) = world.frame(tx, ty)?;
            if fx < 0 || fy < 0 {
                return None;
            }
            let uv = tile_frame::frame_to_uv(fx as u16, fy as u16, meta.w, meta.h)?;
            return Some(TileView { tex: meta.tex, uv });
        }
        self.block(id)
    }

    /// 图集已绑定，但这格没有可采样的已写入帧。
    pub fn missing_block_frame(&self, world: &World, tx: i32, ty: i32) -> bool {
        let id = world.get(tx, ty);
        self.sheets.contains_key(&id.0) && self.block_framed(world, tx, ty).is_none()
    }

    pub fn wall(&self, id: WallId) -> Option<TileView> {
        if let Some(meta) = self.wall_sheets.get(&id.0) {
            return Some(TileView {
                tex: meta.tex,
                uv: meta.fallback_uv,
            });
        }
        let tex = *self.walls.get(&id.0)?;
        Some(TileView { tex, uv: FULL_UV })
    }

    /// 按已写入的墙帧采样。没有帧时不改用图集上的某一格。
    pub fn wall_framed(&self, world: &World, tx: i32, ty: i32) -> Option<TileView> {
        let id = world.get_wall(tx, ty);
        if id == WallId::NONE {
            return None;
        }
        if let Some(meta) = self.wall_sheets.get(&id.0) {
            let (fx, fy) = world.wall_frame(tx, ty)?;
            if fx < 0 || fy < 0 {
                return None;
            }
            let uv = tile_frame::frame_to_uv(fx as u16, fy as u16, meta.w, meta.h)?;
            return Some(TileView { tex: meta.tex, uv });
        }
        self.wall(id)
    }

    /// 墙图集已绑定，但这格没有可采样的已写入帧。
    pub fn missing_wall_frame(&self, world: &World, tx: i32, ty: i32) -> bool {
        let id = world.get_wall(tx, ty);
        id != WallId::NONE
            && self.wall_sheets.contains_key(&id.0)
            && self.wall_framed(world, tx, ty).is_none()
    }

    pub fn halo(&self) -> Option<TileView> {
        Some(TileView {
            tex: self.halo?,
            uv: FULL_UV,
        })
    }

    pub fn white(&self) -> Option<TileView> {
        Some(TileView {
            tex: self.white?,
            uv: FULL_UV,
        })
    }

    pub fn crack(&self) -> Option<TileView> {
        Some(TileView {
            tex: self.crack?,
            uv: FULL_UV,
        })
    }

    pub fn slime(&self) -> Option<TileView> {
        Some(TileView {
            tex: self.slime?,
            uv: self.slime_uv,
        })
    }
}

fn upload_slime_frame(draw: &mut DrawList, path: &Path) -> Option<(TextureId, Rect)> {
    let tex = crate::xnb::decode_texture_file(path).ok()?;
    let image = PixelImage::from_rgba8(tex.width, tex.height, tex.rgba).ok()?;
    let uv = if image.height() >= image.width() * 2 && image.height() % 2 == 0 {
        Rect::new(0.0, 0.5, 1.0, 0.5)
    } else {
        FULL_UV
    };
    let gpu = upload_rgba(draw, image.width(), image.height(), image.into_rgba())?;
    Some((gpu, uv))
}

fn best_cell_uv(image: &PixelImage, grass: bool) -> Option<Rect> {
    let w = image.width();
    let h = image.height();
    let stride = if w >= 18 && h >= 18 && w % 18 == 0 && h % 18 == 0 {
        18
    } else if w >= 16 && h >= 16 && w % 16 == 0 && h % 16 == 0 {
        16
    } else {
        return None;
    };
    let cell = 16u32.min(stride);
    let cols = w / stride;
    let rows = (h / stride).min(if grass { 40 } else { 12 });
    let mut best_score = i32::MIN;
    let mut best = None;
    for row in 0..rows {
        for col in 0..cols {
            let x0 = col * stride;
            let y0 = row * stride;
            let Some(score) = score_cell(image, x0, y0, cell, grass) else {
                continue;
            };
            if score > best_score {
                best_score = score;
                best = Some((x0, y0));
            }
        }
    }
    let (x0, y0) = best?;
    let sprite =
        spark_image::Sprite::new(Rect::new(x0 as f32, y0 as f32, cell as f32, cell as f32));
    sprite.uv(image).ok()
}

fn score_cell(image: &PixelImage, x0: u32, y0: u32, cell: u32, grass: bool) -> Option<i32> {
    let mut opaque = 0i32;
    let mut edge = 0i32;
    let mut samples = 0i32;
    let mut green_top = 0i32;
    let mut green_bot = 0i32;
    let mid = cell / 2;
    let inner = image.pixel(x0 + mid, y0 + mid).ok()?;
    for y in 0..cell {
        for x in 0..cell {
            let px = image.pixel(x0 + x, y0 + y).ok()?;
            if px[3] < 32 {
                continue;
            }
            opaque += 1;
            if grass {
                let hi = (px[0] as i32).max(px[2] as i32);
                let g = px[1] as i32 - hi;
                if y < 6 {
                    green_top += g;
                } else if y + 6 >= cell {
                    green_bot += g;
                }
            }
            let border = x < 2 || y < 2 || x + 2 >= cell || y + 2 >= cell;
            if border {
                edge += (px[0] as i32 - inner[0] as i32).abs()
                    + (px[1] as i32 - inner[1] as i32).abs()
                    + (px[2] as i32 - inner[2] as i32).abs();
                samples += 1;
            }
        }
    }
    if opaque < 24 {
        return None;
    }
    let mean_edge = if samples > 0 { edge / samples } else { 0 };
    let full = (cell as i32) * (cell as i32) - 4;
    if grass {
        let cap = green_top - green_bot;
        if cap < 40 {
            return None;
        }
        return Some(cap + opaque);
    }
    if opaque >= full {
        Some(20_000 - mean_edge)
    } else {
        Some(opaque - mean_edge)
    }
}

fn upload_rgba(draw: &mut DrawList, w: u32, h: u32, rgba: Vec<u8>) -> Option<TextureId> {
    match draw.create_texture(w, h, rgba) {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!(?e, "瓦片纹理上传失败");
            None
        }
    }
}

fn upload_png(draw: &mut DrawList, path: &Path) -> Option<TextureId> {
    let (w, h, rgba) = crate::content_boot::ContentAssets::load_rgba(path)?;
    upload_rgba(draw, w, h, rgba)
}

fn upload_fallback_block(draw: &mut DrawList, id: BlockId) -> Option<TextureId> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    for dy in 0..FALLBACK_CELL {
        for dx in 0..FALLBACK_CELL {
            put_px(&mut rgba, FALLBACK_CELL, dx, dy, pixel_for(id, dx, dy));
        }
    }
    upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, rgba)
}

fn upload_fallback_wall(draw: &mut DrawList, base: Color) -> Option<TextureId> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    paint_wall(&mut rgba, base);
    upload_rgba(draw, FALLBACK_CELL, FALLBACK_CELL, rgba)
}

fn solid_cell(c: Color) -> Vec<u8> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    for dy in 0..FALLBACK_CELL {
        for dx in 0..FALLBACK_CELL {
            put_px(&mut rgba, FALLBACK_CELL, dx, dy, c);
        }
    }
    rgba
}

fn hash_u32(x: u32, y: u32, s: u32) -> u32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(s);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

fn noise(x: u32, y: u32, s: u32) -> f32 {
    (hash_u32(x, y, s) & 255) as f32 / 255.0
}

fn put_px(rgba: &mut [u8], stride_w: u32, x: u32, y: u32, c: Color) {
    if x >= stride_w {
        return;
    }
    let i = ((y * stride_w + x) * 4) as usize;
    if i + 3 >= rgba.len() {
        return;
    }
    rgba[i] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
    rgba[i + 3] = (c.a.clamp(0.0, 1.0) * 255.0) as u8;
}

fn paint_halo_rgba() -> Vec<u8> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    let cx = (FALLBACK_CELL as f32 - 1.0) * 0.5;
    let cy = cx;
    let rad = FALLBACK_CELL as f32 * 0.5;
    for dy in 0..FALLBACK_CELL {
        for dx in 0..FALLBACK_CELL {
            let d = ((dx as f32 - cx).hypot(dy as f32 - cy) / rad).clamp(0.0, 1.0);
            let a = (1.0 - d).powf(1.6);
            put_px(
                &mut rgba,
                FALLBACK_CELL,
                dx,
                dy,
                Color::rgba(1.0, 1.0, 1.0, a),
            );
        }
    }
    rgba
}

fn paint_wall(rgba: &mut [u8], base: Color) {
    for dy in 0..FALLBACK_CELL {
        for dx in 0..FALLBACK_CELL {
            let n = noise(dx, dy, 17);
            let brick = ((dy / 4) + (dx / 8)) % 2 == 0;
            let mut c = if brick {
                Color::rgb(base.r * 0.85, base.g * 0.85, base.b * 0.85)
            } else {
                base
            };
            c.r = (c.r * (0.82 + 0.25 * n)).clamp(0.0, 1.0);
            c.g = (c.g * (0.82 + 0.25 * n)).clamp(0.0, 1.0);
            c.b = (c.b * (0.82 + 0.25 * n)).clamp(0.0, 1.0);
            if dx % 8 == 0 || dy % 4 == 0 {
                c.r *= 0.65;
                c.g *= 0.65;
                c.b *= 0.65;
            }
            put_px(rgba, FALLBACK_CELL, dx, dy, c);
        }
    }
}

fn paint_cracks_rgba() -> Vec<u8> {
    let mut rgba = solid_cell(Color::rgba(0.0, 0.0, 0.0, 0.0));
    let lines = [
        (2, 3, 11, 4),
        (8, 2, 9, 12),
        (3, 10, 13, 11),
        (5, 6, 12, 14),
    ];
    let scale = FALLBACK_CELL as f32 / 16.0;
    for (x1, y1, x2, y2) in lines {
        let steps = 12;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = ((x1 as f32 + (x2 - x1) as f32 * t) * scale).round() as u32;
            let y = ((y1 as f32 + (y2 - y1) as f32 * t) * scale).round() as u32;
            put_px(
                &mut rgba,
                FALLBACK_CELL,
                x.min(FALLBACK_CELL - 1),
                y.min(FALLBACK_CELL - 1),
                Color::rgba(0.05, 0.05, 0.07, 0.75),
            );
        }
    }
    rgba
}

fn paint_slime_rgba() -> Vec<u8> {
    let mut rgba = vec![0u8; (FALLBACK_CELL * FALLBACK_CELL * 4) as usize];
    for dy in 0..FALLBACK_CELL {
        for dx in 0..FALLBACK_CELL {
            let cx = dx as f32 - 7.5;
            let cy = dy as f32 - 9.0;
            let inside = (cx * cx) / 49.0 + (cy * cy) / 25.0 <= 1.0;
            if !inside {
                put_px(
                    &mut rgba,
                    FALLBACK_CELL,
                    dx,
                    dy,
                    Color::rgba(0.0, 0.0, 0.0, 0.0),
                );
                continue;
            }
            let highlight = cx < -2.0 && cy < -1.0;
            let c = if highlight {
                Color::rgb(0.85, 0.55, 0.9)
            } else {
                Color::rgb(0.55, 0.22, 0.55)
            };
            put_px(&mut rgba, FALLBACK_CELL, dx, dy, c);
        }
    }
    put_px(&mut rgba, FALLBACK_CELL, 5, 7, Color::rgb(0.95, 0.92, 1.0));
    put_px(&mut rgba, FALLBACK_CELL, 10, 7, Color::rgb(0.95, 0.92, 1.0));
    put_px(&mut rgba, FALLBACK_CELL, 5, 8, Color::rgb(0.12, 0.08, 0.14));
    put_px(
        &mut rgba,
        FALLBACK_CELL,
        10,
        8,
        Color::rgb(0.12, 0.08, 0.14),
    );
    rgba
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    crate::palette::mix(a, b, t)
}

fn edge_px(x: u32, y: u32) -> bool {
    x == 0 || y == 0 || x == FALLBACK_CELL - 1 || y == FALLBACK_CELL - 1
}

/// 左上受光、右下暗面、底边与外轮廓压暗。裂纹由调用方另画。
fn lit_face(swatch: crate::palette::MaterialSwatch, x: u32, y: u32) -> Color {
    let mut c = if x + y < 8 {
        mix(swatch.base, swatch.highlight, 0.5)
    } else if x + y > 20 {
        mix(swatch.base, swatch.shade, 0.55)
    } else {
        swatch.base
    };
    if y >= FALLBACK_CELL - 2 {
        c = mix(c, swatch.rim, 0.5);
    }
    if edge_px(x, y) {
        c = mix(c, swatch.rim, 0.35);
    }
    c
}

fn pixel_dirt(x: u32, y: u32) -> Color {
    use crate::palette::DIRT;
    let mut c = if y < 8 {
        mix(
            DIRT.base,
            DIRT.highlight,
            if x < 5 && y < 4 { 0.28 } else { 0.06 },
        )
    } else {
        mix(DIRT.base, DIRT.shade, 0.32)
    };
    if y == 5 || y == 6 || y == 11 || y == 12 {
        c = mix(c, DIRT.shade, 0.6);
    }
    if matches!((x, y), (3, 8) | (11, 4) | (7, 13) | (13, 10)) {
        c = mix(c, DIRT.highlight, 0.72);
    }
    if matches!((x, y), (2, 12) | (5, 12) | (12, 9) | (13, 9)) {
        c = mix(c, DIRT.shade, 0.8);
    }
    if y >= FALLBACK_CELL - 2 {
        c = mix(c, DIRT.rim, 0.55);
    }
    if edge_px(x, y) {
        c = mix(c, DIRT.rim, 0.35);
    }
    c
}

fn pixel_grass(x: u32, y: u32) -> Color {
    use crate::palette::{DIRT, GRASS, GRASS_SOIL};
    let reach = 1 + (hash_u32(x, 1, 2) % 4);
    if y < 4 {
        if y < reach {
            if y + 1 == reach {
                mix(GRASS.highlight, GRASS.base, 0.25)
            } else if x % 3 == 0 {
                mix(GRASS.base, GRASS.shade, 0.4)
            } else {
                GRASS.base
            }
        } else {
            mix(GRASS_SOIL.base, GRASS_SOIL.shade, 0.35)
        }
    } else if y < 7 {
        let mut c = mix(GRASS_SOIL.base, DIRT.base, (y - 4) as f32 / 3.0);
        if edge_px(x, y) {
            c = mix(c, GRASS_SOIL.rim, 0.35);
        }
        c
    } else {
        pixel_dirt(x, y)
    }
}

fn pixel_stone(x: u32, y: u32) -> Color {
    use crate::palette::STONE;
    let mut c = if x + y < 9 {
        mix(STONE.base, STONE.highlight, 0.55)
    } else if x + y > 20 {
        mix(STONE.base, STONE.shade, 0.62)
    } else {
        STONE.base
    };
    if x >= 3 && x <= 12 && y + 2 == x {
        c = mix(c, STONE.rim, 0.82);
    }
    if y >= FALLBACK_CELL - 1 {
        c = mix(c, STONE.rim, 0.4);
    }
    if edge_px(x, y) {
        c = mix(c, STONE.rim, 0.4);
    }
    c
}

fn pixel_ore(x: u32, y: u32, vein: crate::palette::MaterialSwatch, slope: i32) -> Color {
    let mut c = pixel_stone(x, y);
    let on = y as i32 == (x as i32) / 2 + slope || y as i32 == 13 - (x as i32) / 3;
    if on && x > 1 && x < 14 {
        c = if x % 4 == 1 {
            vein.highlight
        } else {
            mix(vein.base, vein.shade, 0.35)
        };
    }
    c
}

fn pixel_water(x: u32, y: u32) -> Color {
    use crate::palette::WATER;
    let foam = y <= 1 && (x + y) % 3 != 0;
    if foam {
        Color::rgba(WATER.highlight.r, WATER.highlight.g, WATER.highlight.b, 0.9)
    } else if y < 6 {
        Color::rgba(
            WATER.base.r,
            WATER.base.g,
            WATER.base.b,
            if x % 5 == 2 { 0.35 } else { 0.55 },
        )
    } else {
        Color::rgba(WATER.shade.r, WATER.shade.g, WATER.shade.b, 0.75)
    }
}

fn pixel_wood(x: u32, y: u32) -> Color {
    use crate::palette::WOOD;
    let grain = ((x + y / 3) % 4) as f32 / 4.0;
    let mut c = mix(WOOD.base, WOOD.shade, grain * 0.7);
    if x < 3 && y < 5 {
        c = mix(c, WOOD.highlight, 0.4);
    }
    if x + y > 22 {
        c = mix(c, WOOD.shade, 0.45);
    }
    if x == 5 || x == 10 {
        c = mix(c, WOOD.rim, 0.55);
    }
    if edge_px(x, y) {
        c = mix(c, WOOD.rim, 0.4);
    }
    c
}

fn pixel_leaf(x: u32, y: u32) -> Color {
    use crate::palette::LEAF;
    let n = noise(x, y, 41);
    if n > 0.78 || (x + y) % 7 == 0 && n > 0.45 {
        return Color::rgba(0.0, 0.0, 0.0, 0.0);
    }
    let mut c = if x + y < 10 {
        mix(LEAF.base, LEAF.highlight, 0.45)
    } else if x + y > 20 {
        mix(LEAF.base, LEAF.shade, 0.5)
    } else {
        LEAF.base
    };
    if edge_px(x, y) && n < 0.5 {
        c = mix(c, LEAF.rim, 0.35);
    }
    c
}

fn pixel_for(id: BlockId, x: u32, y: u32) -> Color {
    use crate::palette::{COPPER, IRON, SAND, SNOW};

    let n = noise(x, y, id.0 * 31);
    match id {
        BlockId::DIRT => pixel_dirt(x, y),
        BlockId::GRASS => pixel_grass(x, y),
        BlockId::STONE => pixel_stone(x, y),
        BlockId::WATER => pixel_water(x, y),
        BlockId::SAND => lit_face(SAND, x, y),
        BlockId::SNOW => lit_face(SNOW, x, y),
        BlockId::COPPER_ORE => pixel_ore(x, y, COPPER, 4),
        BlockId::IRON_ORE => pixel_ore(x, y, IRON, 7),
        BlockId::WOOD => pixel_wood(x, y),
        id if id.is_tree() => pixel_wood(x, y),
        BlockId::LEAF => pixel_leaf(x, y),
        BlockId::WORKBENCH => {
            if y < 3 {
                Color::rgb(0.78, 0.55, 0.28)
            } else if x < 2 || x > 13 || y > 14 {
                Color::rgb(0.42, 0.26, 0.12)
            } else {
                Color::rgb(0.62, 0.42, 0.22)
            }
        }
        BlockId::SAPLING => {
            let stem = x >= 7 && x <= 8 && y >= 6;
            let crown = (x as i32 - 8).abs() + (y as i32 - 5).abs() < 5 && y < 9;
            if stem {
                Color::rgb(0.42, 0.26, 0.12)
            } else if crown {
                Color::rgb(0.32, 0.72, 0.28)
            } else {
                Color::rgba(0.0, 0.0, 0.0, 0.0)
            }
        }
        BlockId::TORCH => {
            let stem = x >= 7 && x <= 8 && y >= 6;
            let flame = (x as i32 - 8).abs() <= 2 && y < 7 && y >= 1;
            if flame {
                Color::rgb(1.0, 0.72 + n * 0.2, 0.22)
            } else if stem {
                Color::rgb(0.42, 0.26, 0.12)
            } else {
                Color::rgba(0.0, 0.0, 0.0, 0.0)
            }
        }
        BlockId::PLATFORM => {
            if y >= 9 && y <= 13 {
                mix(
                    Color::rgb(0.62, 0.40, 0.20),
                    Color::rgb(0.80, 0.55, 0.30),
                    n,
                )
            } else {
                Color::rgba(0.0, 0.0, 0.0, 0.0)
            }
        }
        BlockId::CHEST => {
            if x < 2 || y < 2 || x > 13 || y > 13 {
                Color::rgb(0.35, 0.20, 0.08)
            } else if y == 7 || (x >= 7 && x <= 8 && y >= 7 && y <= 9) {
                Color::rgb(0.90, 0.72, 0.28)
            } else {
                Color::rgb(0.70, 0.46, 0.20)
            }
        }
        BlockId::LADDER => {
            let rail = x == 4 || x == 11;
            let rung = y % 4 == 1;
            if rail || (rung && x >= 4 && x <= 11) {
                Color::rgb(0.58, 0.36, 0.16)
            } else {
                Color::rgba(0.0, 0.0, 0.0, 0.0)
            }
        }
        BlockId::ROPE => {
            if x == 7 || x == 8 {
                Color::rgb(0.72, 0.55, 0.28)
            } else if (x == 6 || x == 9) && y % 3 == 0 {
                Color::rgb(0.55, 0.40, 0.18)
            } else {
                Color::rgba(0.0, 0.0, 0.0, 0.0)
            }
        }
        BlockId::FURNACE => {
            if y >= 9 && y <= 12 && x >= 4 && x <= 11 {
                Color::rgb(1.0, 0.42, 0.12)
            } else if n > 0.6 {
                Color::rgb(0.28, 0.26, 0.24)
            } else {
                Color::rgb(0.38, 0.36, 0.34)
            }
        }
        BlockId::BED => {
            if y >= 10 {
                Color::rgb(0.42, 0.26, 0.14)
            } else if y >= 6 {
                Color::rgb(0.82, 0.32, 0.38)
            } else if x < 5 {
                Color::rgb(0.92, 0.88, 0.82)
            } else {
                Color::rgb(0.70, 0.28, 0.32)
            }
        }
        _ => Color::rgb(1.0, 0.0, 1.0),
    }
}
