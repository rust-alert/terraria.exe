//! 裂痕传送：自动进门 + 可选目标 UI。
//!
//! - **自动**：站入 `WARP` 格停留蓄力 → 传到环上下一站
//! - **可选**：靠近裂痕锚按 `E` 打开列表 → 选目标确认传送

use tr_core::BlockId;

use crate::player::{HIT_H, HIT_W, Player};
use crate::world::{TILE, WORLD_H, WORLD_W, World, wrap_delta_x, wrap_tx, wrap_xf};

/// 自动进门蓄满时间（秒）。
pub const AUTO_CHARGE_NEED: f32 = 1.05;
/// 传送后冷却（秒）。
pub const WARP_COOLDOWN: f32 = 1.6;
/// 落地闪光时长。
pub const FLASH_TIME: f32 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalKind {
    Pod,
    Anchor,
}

#[derive(Debug, Clone)]
pub struct PortalGate {
    pub kind: PortalKind,
    pub label: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct PortalState {
    /// 可选传送面板是否打开。
    pub menu_open: bool,
    /// 面板/自动共用的目标下标。
    pub dest_idx: usize,
    /// 自动进门蓄力。
    pub charge: f32,
    pub cooldown: f32,
    pub flash_t: f32,
    pub flash_x: f32,
    pub flash_y: f32,
}

impl Default for PortalState {
    fn default() -> Self {
        Self {
            menu_open: false,
            dest_idx: 0,
            charge: 0.0,
            cooldown: 0.0,
            flash_t: 0.0,
            flash_x: 0.0,
            flash_y: 0.0,
        }
    }
}

impl PortalState {
    pub fn tick_timers(&mut self, dt: f32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.flash_t = (self.flash_t - dt).max(0.0);
    }

    pub fn close_menu(&mut self) {
        self.menu_open = false;
    }

    fn cycle_dest(&mut self, gates: &[PortalGate], here: Option<usize>, dir: i32) {
        let n = gates.len();
        if n < 2 {
            return;
        }
        let mut idx = self.dest_idx % n;
        for _ in 0..n {
            idx = ((idx as i32 + dir).rem_euclid(n as i32)) as usize;
            if Some(idx) != here {
                self.dest_idx = idx;
                return;
            }
        }
    }

    fn ensure_valid_dest(&mut self, gates: &[PortalGate], here: Option<usize>) {
        let n = gates.len();
        if n == 0 {
            return;
        }
        if Some(self.dest_idx % n) == here || self.dest_idx >= n {
            self.cycle_dest(gates, here, 1);
        }
    }
}

/// 本帧输入意图（由 play 组装）。
#[derive(Debug, Clone, Copy, Default)]
pub struct PortalInput {
    /// 切换可选传送 UI（靠近裂痕锚按 E）。
    pub toggle_menu: bool,
    /// 确认传送（Enter / 点击确认）。
    pub confirm: bool,
    /// 关闭 UI。
    pub cancel: bool,
    /// 列表上下（-1/1）。
    pub cycle: i32,
    /// 鼠标点选的目标下标（相对完整 gates 列表）。
    pub pick_gate: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum PortalTick {
    Idle,
    Away,
    NeedPeer,
    Cooling,
    /// 站在裂痕门上自动蓄力。
    AutoCharging,
    MenuOpen,
    Warped {
        dest: String,
        via_auto: bool,
    },
}

/// 收集传送网：舱 + 连通裂痕锚簇。
pub fn collect_gates(world: &World) -> Vec<PortalGate> {
    let mut out = Vec::new();
    let (sx, sy) = world.spawn_pos();
    out.push(PortalGate {
        kind: PortalKind::Pod,
        label: "逃生舱".into(),
        x: sx,
        y: sy,
    });

    let mut visited = vec![false; (WORLD_W * WORLD_H) as usize];
    let mut anchor_n = 0u32;
    for y in 0..WORLD_H {
        for x in 0..WORLD_W {
            let i = (y * WORLD_W + x) as usize;
            if visited[i] || world.get(x, y) != BlockId::WARP {
                continue;
            }
            let mut stack = vec![(x, y)];
            visited[i] = true;
            let mut cells = Vec::new();
            while let Some((cx, cy)) = stack.pop() {
                cells.push((cx, cy));
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = wrap_tx(cx + dx);
                    let ny = cy + dy;
                    if ny < 0 || ny >= WORLD_H {
                        continue;
                    }
                    let ni = (ny * WORLD_W + nx) as usize;
                    if visited[ni] || world.get(nx, ny) != BlockId::WARP {
                        continue;
                    }
                    visited[ni] = true;
                    stack.push((nx, ny));
                }
            }
            let max_y = cells.iter().map(|c| c.1).max().unwrap_or(y);
            let bottom: Vec<_> = cells.iter().copied().filter(|c| c.1 == max_y).collect();
            let avg_x = bottom.iter().map(|c| c.0 as f32).sum::<f32>() / bottom.len() as f32;
            let tx = wrap_tx(avg_x.round() as i32);
            let ty = max_y;
            let px = tx as f32 * TILE + (TILE - HIT_W) * 0.5;
            let py = (ty + 1) as f32 * TILE - HIT_H;
            anchor_n += 1;
            out.push(PortalGate {
                kind: PortalKind::Anchor,
                label: format!("裂痕锚 ·{anchor_n}"),
                x: wrap_xf(px),
                y: py,
            });
        }
    }
    out
}

/// 身体叠在裂痕锚格上（自动传送条件）。
pub fn standing_in_warp(player: &Player, world: &World) -> bool {
    let (px, py, pw, ph) = player.hitbox();
    let cx = px + pw * 0.5;
    let cy = py + ph * 0.55;
    let tx = wrap_tx((cx / TILE).floor() as i32);
    let ty = (cy / TILE).floor() as i32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if world.get(tx + dx, ty + dy) == BlockId::WARP {
                return true;
            }
        }
    }
    false
}

/// 玩家当前所在/紧邻的门下标（自动或开 UI 均可）。
pub fn gate_index_at(player: &Player, world: &World, gates: &[PortalGate]) -> Option<usize> {
    let (px, py, pw, ph) = player.hitbox();
    let cx = px + pw * 0.5;
    let cy = py + ph * 0.5;
    let tx = wrap_tx((cx / TILE).floor() as i32);
    let ty = (cy / TILE).floor() as i32;

    if standing_in_warp(player, world) {
        return nearest_gate_idx(gates, cx, cy, PortalKind::Anchor);
    }
    for dy in -3..=3 {
        for dx in -3..=3 {
            if world.get(tx + dx, ty + dy) == BlockId::POD {
                return nearest_gate_idx(gates, cx, cy, PortalKind::Pod);
            }
        }
    }
    let mut best = None;
    let mut best_d = TILE * 3.2;
    for (i, g) in gates.iter().enumerate() {
        let d = wrap_delta_x(cx, g.x + HIT_W * 0.5).hypot(cy - (g.y + HIT_H * 0.5));
        if d < best_d {
            best_d = d;
            best = Some(i);
        }
    }
    best
}

fn nearest_gate_idx(gates: &[PortalGate], cx: f32, cy: f32, kind: PortalKind) -> Option<usize> {
    let mut best = None;
    let mut best_d = f32::MAX;
    for (i, g) in gates.iter().enumerate() {
        if g.kind != kind {
            continue;
        }
        let d = wrap_delta_x(cx, g.x + HIT_W * 0.5).hypot(cy - (g.y + HIT_H * 0.5));
        if d < best_d {
            best_d = d;
            best = Some(i);
        }
    }
    best
}

fn apply_warp(state: &mut PortalState, player: &mut Player, dest: &PortalGate) -> String {
    let name = dest.label.clone();
    player.x = dest.x;
    player.y = dest.y;
    player.vx = 0.0;
    player.vy = 0.0;
    player.iframes = 1.2;
    state.charge = 0.0;
    state.menu_open = false;
    state.cooldown = WARP_COOLDOWN;
    state.flash_t = FLASH_TIME;
    state.flash_x = dest.x + HIT_W * 0.5;
    state.flash_y = dest.y + HIT_H * 0.5;
    name
}

/// 一帧：自动进门 + 可选 UI。
pub fn tick_portals(
    state: &mut PortalState,
    player: &mut Player,
    world: &World,
    dt: f32,
    input: PortalInput,
) -> PortalTick {
    state.tick_timers(dt);
    let gates = collect_gates(world);
    if gates.len() < 2 {
        state.charge = 0.0;
        if input.toggle_menu {
            return PortalTick::NeedPeer;
        }
        return PortalTick::NeedPeer;
    }

    let here = gate_index_at(player, world, &gates);
    let in_warp = standing_in_warp(player, world);

    // —— 可选 UI ——
    if input.cancel && state.menu_open {
        state.close_menu();
        state.charge = 0.0;
        return PortalTick::Idle;
    }
    if input.toggle_menu {
        if state.menu_open {
            state.close_menu();
        } else if here.is_some() {
            state.menu_open = true;
            state.charge = 0.0;
            state.ensure_valid_dest(&gates, here);
        } else {
            return PortalTick::Away;
        }
    }

    if state.menu_open {
        if here.is_none() {
            state.close_menu();
            return PortalTick::Away;
        }
        state.ensure_valid_dest(&gates, here);
        if let Some(pick) = input.pick_gate {
            if pick < gates.len() && Some(pick) != here {
                state.dest_idx = pick;
            }
        }
        if input.cycle != 0 {
            state.cycle_dest(&gates, here, input.cycle);
        }
        if input.confirm && state.cooldown <= 0.0 {
            let dest = &gates[state.dest_idx % gates.len()];
            if Some(state.dest_idx % gates.len()) != here {
                let name = apply_warp(state, player, dest);
                return PortalTick::Warped {
                    dest: name,
                    via_auto: false,
                };
            }
        }
        return PortalTick::MenuOpen;
    }

    // —— 自动：仅站在裂痕锚格 ——
    if state.cooldown > 0.0 {
        state.charge = 0.0;
        return PortalTick::Cooling;
    }
    if in_warp {
        let here_i = here.unwrap_or(0);
        // 自动目标：环上下一站
        let auto_dest = (here_i + 1) % gates.len();
        state.dest_idx = auto_dest;
        state.charge = (state.charge + dt).min(AUTO_CHARGE_NEED);
        let dest = &gates[auto_dest];
        if state.charge >= AUTO_CHARGE_NEED {
            let name = apply_warp(state, player, dest);
            return PortalTick::Warped {
                dest: name,
                via_auto: true,
            };
        }
        return PortalTick::AutoCharging;
    }

    state.charge = (state.charge - dt * 3.0).max(0.0);
    if here.is_some() {
        PortalTick::Idle
    } else {
        PortalTick::Away
    }
}

/// demo：立刻传到下一站。
pub fn force_warp_next(
    player: &mut Player,
    world: &World,
    state: &mut PortalState,
) -> Option<String> {
    let gates = collect_gates(world);
    if gates.len() < 2 {
        return None;
    }
    let here = gate_index_at(player, world, &gates).unwrap_or(0);
    let dest_i = (here + 1) % gates.len();
    state.dest_idx = dest_i;
    Some(apply_warp(state, player, &gates[dest_i]))
}
