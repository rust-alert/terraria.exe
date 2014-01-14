//! 钩爪：发射 → 挂点 → 牵引。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::ItemId;

use crate::player::Player;
use crate::world::{TILE, WORLD_H, World, world_pixel_w, wrap_delta_x, wrap_tx, wrap_xf};

const HOOK_SPEED: f32 = TILE * 38.0;
const MAX_LEN: f32 = TILE * 18.0;
const PULL_SPEED: f32 = TILE * 24.0;
const LATCH_STICK: f32 = TILE * 0.7;

#[derive(Debug, Clone)]
pub enum GrapplePhase {
    Flying { x: f32, y: f32, vx: f32, vy: f32 },
    Latched { ax: f32, ay: f32 },
}

#[derive(Debug, Clone)]
pub struct Grapple {
    pub phase: GrapplePhase,
}

impl Grapple {
    pub fn tip(&self) -> (f32, f32) {
        match self.phase {
            GrapplePhase::Flying { x, y, .. } => (x, y),
            GrapplePhase::Latched { ax, ay } => (ax, ay),
        }
    }

    pub fn is_latched(&self) -> bool {
        matches!(self.phase, GrapplePhase::Latched { .. })
    }
}

/// 手持钩爪时按 Q：有绳则收回，否则朝瞄准点发射。
pub fn try_use(player: &mut Player, aim_x: f32, aim_y: f32) -> Option<String> {
    if !player.selected_item().is_grapple() {
        return None;
    }
    if player.inv.get(ItemId::GRAPPLE) == 0 {
        return Some("没有钩爪".into());
    }
    if player.grapple.is_some() {
        player.grapple = None;
        return Some("收回钩爪".into());
    }
    let (px, py, pw, ph) = player.hitbox();
    let ox = px + pw * 0.5;
    let oy = py + ph * 0.35;
    let dx = wrap_delta_x(ox, aim_x);
    let dy = aim_y - oy;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let vx = dx / len * HOOK_SPEED;
    let vy = dy / len * HOOK_SPEED;
    player.facing = if dx >= 0.0 { 1.0 } else { -1.0 };
    player.grapple = Some(Grapple {
        phase: GrapplePhase::Flying {
            x: ox,
            y: oy,
            vx,
            vy,
        },
    });
    player.mark_swing_for(0.18);
    Some("抛出钩爪".into())
}

/// 挂点时覆盖本帧速度（在 `move_axis` 之前调用）。
pub fn apply_pull(player: &mut Player) {
    let Some(g) = player.grapple.as_ref() else {
        return;
    };
    let GrapplePhase::Latched { ax, ay } = g.phase else {
        return;
    };
    let (px, py, pw, ph) = player.hitbox();
    let ox = px + pw * 0.5;
    let oy = py + ph * 0.35;
    let dx = wrap_delta_x(ox, ax);
    let dy = ay - oy;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > MAX_LEN * 1.15 {
        player.grapple = None;
        return;
    }
    if dist > LATCH_STICK {
        let nx = dx / dist;
        let ny = dy / dist;
        player.vx = nx * PULL_SPEED;
        player.vy = ny * PULL_SPEED;
        player.on_ground = false;
        player.fall_start_y = None;
    } else {
        player.vx *= 0.55;
        player.vy *= 0.55;
        player.fall_start_y = None;
    }
}

/// 推进飞钩并检测挂墙（在位移之后调用）。
pub fn advance_hook(player: &mut Player, world: &World, dt: f32) {
    if player.selected_item() != ItemId::GRAPPLE {
        player.grapple = None;
        return;
    }
    let Some(g) = player.grapple.take() else {
        return;
    };
    let (px, py, pw, ph) = player.hitbox();
    let ox = px + pw * 0.5;
    let oy = py + ph * 0.35;
    let world_h_px = WORLD_H as f32 * TILE;

    match g.phase {
        GrapplePhase::Flying {
            mut x,
            mut y,
            vx,
            vy,
        } => {
            let steps = ((HOOK_SPEED * dt) / (TILE * 0.2)).ceil().max(1.0) as i32;
            let step_dt = dt / steps as f32;
            let mut latched = None;
            for _ in 0..steps {
                x = wrap_xf(x + vx * step_dt);
                y += vy * step_dt;
                if y < 0.0 || y >= world_h_px {
                    player.grapple = None;
                    return;
                }
                let dist = wrap_delta_x(ox, x).hypot(y - oy);
                if dist > MAX_LEN {
                    player.grapple = None;
                    return;
                }
                let tx = wrap_tx((x / TILE).floor() as i32);
                let ty = (y / TILE).floor() as i32;
                if world.in_bounds(tx, ty) && world.get(tx, ty).blocks_motion() {
                    latched = Some((
                        wrap_xf(tx as f32 * TILE + TILE * 0.5),
                        ty as f32 * TILE + TILE * 0.5,
                    ));
                    break;
                }
            }
            if let Some((ax, ay)) = latched {
                player.fall_start_y = None;
                player.grapple = Some(Grapple {
                    phase: GrapplePhase::Latched { ax, ay },
                });
            } else {
                player.grapple = Some(Grapple {
                    phase: GrapplePhase::Flying { x, y, vx, vy },
                });
            }
        }
        GrapplePhase::Latched { ax, ay } => {
            let dist = wrap_delta_x(ox, ax).hypot(ay - oy);
            if dist > MAX_LEN * 1.15 {
                player.grapple = None;
            } else {
                player.grapple = Some(Grapple {
                    phase: GrapplePhase::Latched { ax, ay },
                });
            }
        }
    }
}

/// 画绳索与钩头。
pub fn draw(player: &Player, draw: &mut DrawList, cam_x: f32, cam_y: f32) {
    let Some(g) = player.grapple.as_ref() else {
        return;
    };
    let (tx, ty) = g.tip();
    let (px, py, pw, ph) = player.hitbox();
    let ox = px + pw * 0.5;
    let oy = py + ph * 0.35;

    let w = world_pixel_w();
    let mut sx0 = ox - cam_x;
    let mut sx1 = tx - cam_x;
    if sx0 > w * 0.5 {
        sx0 -= w;
    } else if sx0 < -w * 0.5 {
        sx0 += w;
    }
    if sx1 > w * 0.5 {
        sx1 -= w;
    } else if sx1 < -w * 0.5 {
        sx1 += w;
    }
    let sy0 = oy - cam_y;
    let sy1 = ty - cam_y;

    let segs = 18;
    let rope = Color::rgb(0.72, 0.55, 0.32);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let x = sx0 + (sx1 - sx0) * t;
        let y = sy0 + (sy1 - sy0) * t;
        draw.fill_rect(Rect::new(x - 1.0, y - 1.0, 2.0, 2.0), rope);
    }
    let hook = if g.is_latched() {
        Color::rgb(0.95, 0.85, 0.35)
    } else {
        Color::rgb(0.85, 0.88, 0.95)
    };
    draw.fill_rect(Rect::new(sx1 - 3.0, sy1 - 3.0, 6.0, 6.0), hook);
}
