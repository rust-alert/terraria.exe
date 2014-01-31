//! 森林树：`Tree_Tops_0` 树冠 + `Tree_Branches_0` 侧枝。
//!
//! 世界里树仍是一列 `WOOD` 加周围 `LEAF`（碰撞与砍伐不变）。
//! 画出时不再铺方块，改贴树冠与树干。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};
use tr_core::BlockId;

use crate::world::{TILE, World, screen_len, screen_of, x_in_bounds};

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

/// 已上传的森林树部件。缺文件时不替换方块树。
#[derive(Default)]
pub struct TreeAtlas {
    tops: Option<Sheet>,
    branches: Option<Sheet>,
    ready: bool,
}

impl TreeAtlas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ready(&self) -> bool {
        self.tops.is_some()
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
        self.tops = upload(draw, &images.join("Tree_Tops_0.xnb"));
        self.branches = upload(draw, &images.join("Tree_Branches_0.xnb"));
        tracing::info!(
            tops = self.tops.is_some(),
            branches = self.branches.is_some(),
            "森林树贴图已上传"
        );
    }

    pub fn paint<F>(
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
        let Some(tops) = &self.tops else {
            return;
        };
        let x_lo = x0 - 6;
        let x_hi = x1 + 6;
        for tx in x_lo..=x_hi {
            let Some(trunk) = trunk_at(world, tx) else {
                continue;
            };
            if trunk.soil < y0 - 2 || trunk.top > y1 + 6 {
                continue;
            }
            let frame = (tx.rem_euclid(3)) as u32;
            let tint = tint_at(tx, trunk.top);
            for y in trunk.top..trunk.soil {
                if y < y0 - 1 || y > y1 + 1 {
                    continue;
                }
                paint_bark(draw, tops, frame, tx, y, cam_x, cam_y, tint);
                if let Some(branches) = &self.branches {
                    if y > trunk.top && y + 1 < trunk.soil && (tx + y).rem_euclid(3) == 0 {
                        let side = if tx.rem_euclid(2) == 0 { 0 } else { 1 };
                        let row = (y.rem_euclid(3)) as u32;
                        paint_branch(draw, branches, side, row, tx, y, cam_x, cam_y, tint);
                    }
                }
            }
            paint_top(draw, tops, frame, tx, trunk.top, cam_x, cam_y, tint);
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

/// 这一格是自然树的树干或树冠占位，绘制时跳过方块。
pub fn hides_block(world: &World, tx: i32, ty: i32) -> bool {
    match world.get(tx, ty) {
        BlockId::WOOD => trunk_at(world, tx).is_some(),
        BlockId::LEAF => {
            for dx in -3..=3 {
                if let Some(trunk) = trunk_at(world, tx + dx) {
                    if (ty - trunk.top).abs() <= 4 {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

struct Trunk {
    top: i32,
    soil: i32,
}

fn trunk_at(world: &World, tx: i32) -> Option<Trunk> {
    if !x_in_bounds(tx) {
        return None;
    }
    let soil = world.surface_at(tx);
    if world.get(tx, soil - 1) != BlockId::WOOD {
        return None;
    }
    let mut top = soil - 1;
    while top > 1 && world.get(tx, top - 1) == BlockId::WOOD {
        top -= 1;
    }
    let mut leafy = false;
    for dy in -2..=1 {
        for dx in -2..=2 {
            if world.get(tx + dx, top + dy) == BlockId::LEAF {
                leafy = true;
            }
        }
    }
    if !leafy {
        return None;
    }
    Some(Trunk { top, soil })
}

fn paint_bark(
    draw: &mut DrawList,
    sheet: &Sheet,
    frame: u32,
    tx: i32,
    ty: i32,
    cam_x: f32,
    cam_y: f32,
    tint: Color,
) {
    // 树冠底边正中的树皮，竖着重复成树干。
    let x = frame * TOP_STRIDE + 32;
    let y = 64u32;
    let uv = Rect::new(
        x as f32 / sheet.w.max(1) as f32,
        y as f32 / sheet.h.max(1) as f32,
        16.0 / sheet.w.max(1) as f32,
        16.0 / sheet.h.max(1) as f32,
    );
    let sy = screen_of(ty as f32 * TILE, cam_y);
    let w = screen_len(TILE * 0.55);
    let x0 = screen_of(tx as f32 * TILE, cam_x) + (screen_len(TILE) - w) * 0.5;
    draw.tex_rect(sheet.tex, Rect::new(x0, sy, w, screen_len(TILE)), uv, tint);
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
