//! 自然树：权威格是不挡移动的 `TREE` 树干；树冠与侧枝是特殊绘制。
//!
//! 绘制落在实心物块之前（背景物）。不使用假 `LEAF` 格，也不用整图遮盖错误逻辑块。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};
use tr_core::BlockId;

use crate::tile_frame::CELL;
use crate::world::{TILE, WORLD_H, World, screen_len, screen_of, x_in_bounds};

/// 树图集帧步长（`Tiles_5`）：16 画面 + 6 间距。
const TREE_STRIDE: u32 = 22;
/// `Tree_Tops_0`：三帧，步长 82，可画 80。
const TOP_STRIDE: u32 = 82;
const TOP_CELL: u32 = 80;
const TOP_SHEET_W: f32 = 246.0;
const TOP_SHEET_H: f32 = 82.0;
/// `Tree_Branches_0`：左右两列、三行，步长 42，可画 40。
const BRANCH_STRIDE: u32 = 42;
const BRANCH_CELL: u32 = 40;
const BRANCH_SHEET_W: f32 = 84.0;
const BRANCH_SHEET_H: f32 = 126.0;

struct Sheet {
    tex: TextureId,
    w: u32,
    h: u32,
}

/// 已上传的森林树部件。
#[derive(Default)]
pub struct TreeAtlas {
    /// `Tiles_5`：树干格。
    trunks: Option<Sheet>,
    tops: Option<Sheet>,
    branches: Option<Sheet>,
    ready: bool,
}

impl TreeAtlas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ready(&self) -> bool {
        self.trunks.is_some() || self.tops.is_some()
    }

    pub fn ensure(&mut self, draw: &mut DrawList, install: Option<&std::path::Path>) {
        if self.ready {
            return;
        }
        self.ready = true;
        let Some(root) = install else {
            return;
        };
        let images = root.join("Content").join("Images");
        self.trunks = upload(draw, &images.join("Tiles_5.xnb"));
        self.tops = upload(draw, &images.join("Tree_Tops_0.xnb"));
        self.branches = upload(draw, &images.join("Tree_Branches_0.xnb"));
        tracing::info!(
            trunks = self.trunks.is_some(),
            tops = self.tops.is_some(),
            branches = self.branches.is_some(),
            "森林树贴图已上传"
        );
    }

    /// 画在实心物块之前：树干 → 侧枝 → 树冠。
    pub fn paint_behind_solids<F>(
        &self,
        draw: &mut DrawList,
        world: &World,
        cam_x: f32,
        cam_y: f32,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        tint_at: F,
    ) where
        F: Fn(i32, i32) -> Color,
    {
        let x_lo = x0 - 6;
        let x_hi = x1 + 6;
        for tx in x_lo..=x_hi {
            let Some(trunk) = trunk_at(world, tx) else {
                continue;
            };
            if trunk.soil < y0 - 2 || trunk.top > y1 + 6 {
                continue;
            }
            let style = (tx.rem_euclid(3)) as u32;
            let tint = tint_at(tx, trunk.top);

            for y in trunk.top..trunk.soil {
                if y < y0 - 1 || y > y1 + 1 {
                    continue;
                }
                paint_trunk_cell(
                    draw,
                    self.trunks.as_ref(),
                    self.tops.as_ref(),
                    style,
                    tx,
                    y,
                    trunk.top,
                    trunk.soil,
                    cam_x,
                    cam_y,
                    tint,
                );
                if let Some(branches) = &self.branches {
                    if y > trunk.top && y + 1 < trunk.soil && (tx + y).rem_euclid(3) == 0 {
                        let side = if tx.rem_euclid(2) == 0 { 0 } else { 1 };
                        let row = (y.rem_euclid(3)) as u32;
                        paint_branch(draw, branches, side, row, tx, y, cam_x, cam_y, tint);
                    }
                }
            }

            if let Some(tops) = &self.tops {
                paint_top(draw, tops, style, tx, trunk.top, cam_x, cam_y, tint);
            }
        }
    }
}

fn upload(draw: &mut DrawList, path: &std::path::Path) -> Option<Sheet> {
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "缺少树贴图");
        return None;
    }
    let tex = crate::xnb::decode_texture_file(path).ok()?;
    if tex.width == 0 || tex.height == 0 {
        return None;
    }
    match draw.create_texture(tex.width, tex.height, tex.rgba) {
        Ok(id) => Some(Sheet {
            tex: id,
            w: tex.width,
            h: tex.height,
        }),
        Err(e) => {
            tracing::warn!(?e, "树贴图上传失败");
            None
        }
    }
}

/// 自然树干格由本模块绘制时，跳过通用方块通道。
pub fn hides_block(world: &World, tx: i32, ty: i32) -> bool {
    world.get(tx, ty) == BlockId::TREE && trunk_at(world, tx).is_some()
}

struct Trunk {
    top: i32,
    soil: i32,
}

fn trunk_at(world: &World, tx: i32) -> Option<Trunk> {
    if !x_in_bounds(tx) {
        return None;
    }
    // 从地表往上找连续 `TREE`，不依赖生成时的噪声地表函数是否仍准确。
    let mut y = 0;
    let mut found_soil = None;
    while y < WORLD_H {
        let id = world.get(tx, y);
        if matches!(
            id,
            BlockId::GRASS | BlockId::DIRT | BlockId::SNOW | BlockId::SAND | BlockId::STONE
        ) {
            // 可能的土壤：上方若是树干则确认。
            if y > 0 && world.get(tx, y - 1) == BlockId::TREE {
                found_soil = Some(y);
                break;
            }
        }
        y += 1;
    }
    let soil = found_soil.or_else(|| {
        let s = world.surface_at(tx);
        if world.get(tx, s - 1) == BlockId::TREE {
            Some(s)
        } else {
            None
        }
    })?;
    if world.get(tx, soil - 1) != BlockId::TREE {
        return None;
    }
    let mut top = soil - 1;
    while top > 1 && world.get(tx, top - 1) == BlockId::TREE {
        top -= 1;
    }
    Some(Trunk { top, soil })
}

fn paint_trunk_cell(
    draw: &mut DrawList,
    trunks: Option<&Sheet>,
    tops_fallback: Option<&Sheet>,
    style: u32,
    tx: i32,
    ty: i32,
    top: i32,
    soil: i32,
    cam_x: f32,
    cam_y: f32,
    tint: Color,
) {
    let sx = screen_of(tx as f32 * TILE, cam_x);
    let sy = screen_of(ty as f32 * TILE, cam_y);
    let tile_px = screen_len(TILE);

    if let Some(sheet) = trunks {
        let (fu, fv) = trunk_frame(ty, top, soil, style);
        let uv = cell_uv(sheet, fu, fv);
        draw.tex_rect(sheet.tex, Rect::new(sx, sy, tile_px, tile_px), uv, tint);
        return;
    }

    // 无 `Tiles_5` 时：从树冠图裁树皮，宽度收窄，仍画在背景层。
    if let Some(sheet) = tops_fallback {
        let x = style * TOP_STRIDE + 32;
        let y = 64u32;
        let uv = Rect::new(
            x as f32 / sheet.w.max(1) as f32,
            y as f32 / sheet.h.max(1) as f32,
            16.0 / sheet.w.max(1) as f32,
            16.0 / sheet.h.max(1) as f32,
        );
        let w = screen_len(TILE * 0.55);
        let x0 = sx + (tile_px - w) * 0.5;
        draw.tex_rect(sheet.tex, Rect::new(x0, sy, w, tile_px), uv, tint);
        return;
    }

    let w = tile_px * 0.45;
    draw.fill_rect(
        Rect::new(sx + (tile_px - w) * 0.5, sy, w, tile_px),
        Color::rgba(tint.r * 0.55, tint.g * 0.35, tint.b * 0.18, tint.a),
    );
}

fn trunk_frame(ty: i32, top: i32, soil: i32, style: u32) -> (u32, u32) {
    let variant = style % 3;
    if ty + 1 == soil {
        // 根部 / 树桩一带。
        (variant * TREE_STRIDE, TREE_STRIDE * 2)
    } else if ty == top {
        // 树冠下第一节。
        (variant * TREE_STRIDE, 0)
    } else {
        // 中段树干。
        (0, variant * TREE_STRIDE)
    }
}

fn cell_uv(sheet: &Sheet, px: u32, py: u32) -> Rect {
    Rect::new(
        px as f32 / sheet.w.max(1) as f32,
        py as f32 / sheet.h.max(1) as f32,
        CELL as f32 / sheet.w.max(1) as f32,
        CELL as f32 / sheet.h.max(1) as f32,
    )
}

fn paint_top(
    draw: &mut DrawList,
    sheet: &Sheet,
    frame: u32,
    tx: i32,
    top: i32,
    cam_x: f32,
    cam_y: f32,
    tint: Color,
) {
    let u = (frame * TOP_STRIDE) as f32 / TOP_SHEET_W;
    let uv = Rect::new(
        u,
        0.0,
        TOP_CELL as f32 / TOP_SHEET_W,
        TOP_CELL as f32 / TOP_SHEET_H,
    );
    let sprite = screen_len(TILE * (TOP_CELL as f32 / 16.0));
    let foot_x = screen_of(tx as f32 * TILE + TILE * 0.5, cam_x);
    let foot_y = screen_of((top as f32 + 1.0) * TILE, cam_y);
    draw.tex_rect(
        sheet.tex,
        Rect::new(foot_x - sprite * 0.5, foot_y - sprite, sprite, sprite),
        uv,
        tint,
    );
}

fn paint_branch(
    draw: &mut DrawList,
    sheet: &Sheet,
    side: u32,
    row: u32,
    tx: i32,
    ty: i32,
    cam_x: f32,
    cam_y: f32,
    tint: Color,
) {
    let u = (side * BRANCH_STRIDE) as f32 / BRANCH_SHEET_W;
    let v = (row * BRANCH_STRIDE) as f32 / BRANCH_SHEET_H;
    let uv = Rect::new(
        u,
        v,
        BRANCH_CELL as f32 / BRANCH_SHEET_W,
        BRANCH_CELL as f32 / BRANCH_SHEET_H,
    );
    let sprite = screen_len(TILE * (BRANCH_CELL as f32 / 16.0));
    let tile_x = screen_of(tx as f32 * TILE, cam_x);
    let tile_y = screen_of(ty as f32 * TILE, cam_y);
    let tile_px = screen_len(TILE);
    let x = if side == 0 {
        tile_x + tile_px - sprite
    } else {
        tile_x
    };
    let y = tile_y + (tile_px - sprite) * 0.5;
    draw.tex_rect(sheet.tex, Rect::new(x, y, sprite, sprite), uv, tint);
}
