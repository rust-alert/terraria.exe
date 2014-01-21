//! 瞄准格预览：在点击前显示挖/放/交互合法性。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{BlockId, ItemId, WeaponKind};

use crate::player::{HIT_H, HIT_W, Player};
use crate::world::{TILE, World, tile_x_near, wrap_delta_x};

/// 与挖掘/放置一致的最大交互距离（格）。
pub const REACH_TILES: f32 = 4.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimKind {
    /// 超出距离：虚线灰框。
    OutOfRange,
    /// 可挖前景。
    MineOk,
    /// 可挖但工具不足。
    MineNeedTool,
    /// 透过前景挖墙。
    MineWall,
    /// 可放置。
    PlaceOk,
    /// 与角色重叠。
    PlaceOverlap,
    /// 缺少支撑。
    PlaceNoSupport,
    /// 手持物不可放，且目标非可挖。
    Idle,
    /// 箱/床/制作站等可交互设施。
    Interact,
    /// 手持武器时的攻击朝向格。
    Attack,
}

#[derive(Debug, Clone, Copy)]
pub struct AimPreview {
    pub tx: i32,
    pub ty: i32,
    pub kind: AimKind,
}

pub fn evaluate(world: &World, player: &Player, aim_wx: f32, aim_wy: f32) -> Option<AimPreview> {
    let tx = (aim_wx / TILE).floor() as i32;
    let ty = (aim_wy / TILE).floor() as i32;
    if !world.in_bounds(tx, ty) {
        return None;
    }

    let cx = player.x + HIT_W * 0.5;
    let cy = player.y + HIT_H * 0.5;
    let bx = tile_x_near(tx, cx) + TILE * 0.5;
    let by = ty as f32 * TILE + TILE * 0.5;
    let dist = (wrap_delta_x(cx, bx).hypot(by - cy)) / TILE;
    if dist > REACH_TILES {
        return Some(AimPreview {
            tx,
            ty,
            kind: AimKind::OutOfRange,
        });
    }

    let block = world.get(tx, ty);
    let item = player.selected_item();

    if is_interactable(world, tx, ty) {
        return Some(AimPreview {
            tx,
            ty,
            kind: AimKind::Interact,
        });
    }

    if item.is_hammer() {
        if can_mine_wall_through(block) && world.get_wall(tx, ty).mineable() {
            return Some(AimPreview {
                tx,
                ty,
                kind: AimKind::MineWall,
            });
        }
        return Some(AimPreview {
            tx,
            ty,
            kind: AimKind::Idle,
        });
    }

    if let Some(wpn) = item.weapon() {
        if matches!(
            wpn.kind,
            WeaponKind::Melee | WeaponKind::Ranged | WeaponKind::Magic
        ) && item.as_block().is_none()
        {
            return Some(AimPreview {
                tx,
                ty,
                kind: AimKind::Attack,
            });
        }
    }

    if item.as_block().is_some() || item == ItemId::SAPLING {
        return Some(AimPreview {
            tx,
            ty,
            kind: place_kind(world, player, tx, ty, item),
        });
    }

    if block.mineable() {
        let kind = if let Some(need) = block.mine_power_need() {
            let power = item.mine_power().unwrap_or(0);
            if power < need {
                AimKind::MineNeedTool
            } else {
                AimKind::MineOk
            }
        } else {
            AimKind::MineOk
        };
        return Some(AimPreview { tx, ty, kind });
    }

    if can_mine_wall_through(block) && world.get_wall(tx, ty).mineable() {
        return Some(AimPreview {
            tx,
            ty,
            kind: AimKind::MineWall,
        });
    }

    Some(AimPreview {
        tx,
        ty,
        kind: AimKind::Idle,
    })
}

fn place_kind(world: &World, player: &Player, tx: i32, ty: i32, item: ItemId) -> AimKind {
    let target = world.get(tx, ty);
    if !matches!(
        target,
        BlockId::AIR
            | BlockId::LEAF
            | BlockId::SAPLING
            | BlockId::TORCH
            | BlockId::LADDER
            | BlockId::ROPE
            | BlockId::WATER
    ) {
        return AimKind::Idle;
    }
    if item == ItemId::SAPLING {
        if world.can_plant_sapling(tx, ty) {
            return AimKind::PlaceOk;
        }
        return AimKind::PlaceNoSupport;
    }
    let Some(block) = item.as_block() else {
        return AimKind::Idle;
    };
    let bx = tile_x_near(tx, player.x + HIT_W * 0.5);
    let by = ty as f32 * TILE;
    if block.blocks_motion() && aabb_overlap(player.x, player.y, HIT_W, HIT_H, bx, by, TILE, TILE) {
        return AimKind::PlaceOverlap;
    }
    if block.blocks_motion() && !has_support(world, tx, ty) {
        return AimKind::PlaceNoSupport;
    }
    AimKind::PlaceOk
}

fn has_support(world: &World, tx: i32, ty: i32) -> bool {
    const DIRS: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    for (dx, dy) in DIRS {
        let n = world.get(tx + dx, ty + dy);
        if n.blocks_motion() {
            return true;
        }
    }
    false
}

fn is_interactable(world: &World, tx: i32, ty: i32) -> bool {
    matches!(
        world.get(tx, ty),
        BlockId::CHEST | BlockId::BED | BlockId::WORKBENCH | BlockId::FURNACE
    )
}

fn can_mine_wall_through(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::AIR
            | BlockId::LEAF
            | BlockId::SAPLING
            | BlockId::TORCH
            | BlockId::LADDER
            | BlockId::ROPE
            | BlockId::PLATFORM
            | BlockId::WATER
    )
}

fn aabb_overlap(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn kind_color(kind: AimKind) -> (Color, bool) {
    match kind {
        AimKind::OutOfRange => (Color::rgba(0.65, 0.68, 0.72, 0.55), true),
        AimKind::MineOk => (Color::rgba(1.0, 1.0, 1.0, 0.85), false),
        AimKind::MineNeedTool => (Color::rgba(0.95, 0.25, 0.22, 0.9), false),
        AimKind::MineWall => (Color::rgba(0.85, 0.88, 0.95, 0.7), false),
        AimKind::PlaceOk => (Color::rgba(0.35, 0.92, 0.45, 0.55), false),
        AimKind::PlaceOverlap => (Color::rgba(0.95, 0.25, 0.22, 0.55), false),
        AimKind::PlaceNoSupport => (Color::rgba(0.95, 0.55, 0.15, 0.55), false),
        AimKind::Interact => (Color::rgba(0.25, 0.9, 0.95, 0.85), false),
        AimKind::Attack => (Color::rgba(1.0, 0.85, 0.35, 0.75), false),
        AimKind::Idle => (Color::rgba(1.0, 1.0, 1.0, 0.35), false),
    }
}

/// 在世界坐标相机下绘制瞄准反馈。
pub fn paint(draw: &mut DrawList, cam_x: f32, cam_y: f32, preview: AimPreview) {
    let sx = preview.tx as f32 * TILE - cam_x;
    let sy = preview.ty as f32 * TILE - cam_y;
    let (color, dashed) = kind_color(preview.kind);
    let t = 2.0;

    match preview.kind {
        AimKind::PlaceOk | AimKind::PlaceOverlap | AimKind::PlaceNoSupport => {
            draw.fill_rect(Rect::new(sx, sy, TILE, TILE), color);
            stroke_rect(
                draw,
                sx,
                sy,
                TILE,
                TILE,
                t,
                Color::rgba(color.r, color.g, color.b, 0.95),
                dashed,
            );
        }
        _ => {
            stroke_rect(draw, sx, sy, TILE, TILE, t, color, dashed);
        }
    }
}

fn stroke_rect(
    draw: &mut DrawList,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    t: f32,
    color: Color,
    dashed: bool,
) {
    if !dashed {
        draw.fill_rect(Rect::new(x, y, w, t), color);
        draw.fill_rect(Rect::new(x, y + h - t, w, t), color);
        draw.fill_rect(Rect::new(x, y, t, h), color);
        draw.fill_rect(Rect::new(x + w - t, y, t, h), color);
        return;
    }
    // 虚线：每边两段。
    let seg = w * 0.28;
    let gap = w * 0.12;
    draw.fill_rect(Rect::new(x, y, seg, t), color);
    draw.fill_rect(Rect::new(x + seg + gap, y, seg, t), color);
    draw.fill_rect(Rect::new(x + w - seg * 2.0 - gap, y + h - t, seg, t), color);
    draw.fill_rect(Rect::new(x + w - seg, y + h - t, seg, t), color);
    draw.fill_rect(Rect::new(x, y, t, seg), color);
    draw.fill_rect(Rect::new(x, y + seg + gap, t, seg), color);
    draw.fill_rect(Rect::new(x + w - t, y + h - seg * 2.0 - gap, t, seg), color);
    draw.fill_rect(Rect::new(x + w - t, y + h - seg, t, seg), color);
}
