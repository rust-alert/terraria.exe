//! 玩家：碰撞、采集、放置、制作与传送。
//!
//! 精灵绘制见 `sprite`；背包见 `inv`；移动见 `movement`。

mod inv;
mod movement;
mod sprite;

pub use inv::{BAG_POCKET_SLOTS, HOTBAR_LEN, Inventory, ItemStack};
pub use sprite::PlayerAtlas;

use spark_input::Input;
use tr_core::{BlockId, DamageHit, ItemId, ResistProfile, resolve_damage};

use crate::craft::{CraftStation, RECIPES};
use crate::grapple::Grapple;
use crate::world::{
    TILE, WORLD_H, World, tile_x_near, world_pixel_w, wrap_delta_x, wrap_tx, wrap_xf,
};

pub const HIT_W: f32 = TILE * (20.0 / 16.0);
pub const HIT_H: f32 = TILE * (42.0 / 16.0);
/// 徒手每击伤害。泥土 50→5 击，石头 100→10 击。
const HAND_DAMAGE: u16 = 10;
/// 挥击间隔（秒），按住左键连续挖。
const SWING_INTERVAL: f32 = 0.22;
pub const PLAYER_MAX_HP: f32 = 100.0;
pub const PLAYER_MAX_MP: f32 = 100.0;
const MANA_REGEN: f32 = 16.0;

#[derive(Debug, Clone)]
pub struct Player {
    /// 碰撞盒左上角（世界坐标，亚像素浮点）。
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub inv: Inventory,
    pub hp: f32,
    pub max_hp: f32,
    pub mp: f32,
    pub max_mp: f32,
    /// 受伤无敌剩余时间。
    pub iframes: f32,
    pub pending_respawn: bool,
    /// `facing >= 0` 朝右，否则朝左。
    pub facing: f32,
    /// 累积步行距离，驱动腿部帧。
    pub(crate) walk_phase: f32,
    /// 下次可挥击的倒计时。
    swing_cd: f32,
    /// 离地时脚底世界 Y；落地按下落高度结算伤害。
    pub(crate) fall_start_y: Option<f32>,
    /// 耗魔后短暂停回蓝。
    mana_regen_cd: f32,
    /// 家园床重生点（世界像素）；`None` 则回舱。
    pub home_spawn: Option<(f32, f32)>,
    /// 活动中的钩爪（飞行或已挂点）。
    pub grapple: Option<Grapple>,
    /// 空中尚可使用的额外跳跃次数（饰品刷新）。
    pub extra_jumps: u8,
    /// 本帧是否刚落地（供尘土特效消费后清零）。
    pub just_landed: bool,
    /// 离地后仍可起跳的土狼剩余时间。
    pub(crate) coyote_t: f32,
    /// 提前按下的跳跃输入缓冲。
    pub(crate) jump_buffer_t: f32,
    /// 单向平台下穿忽略碰撞的剩余时间。
    pub(crate) drop_through_t: f32,
    /// 本段跳跃是否仍等待「松键削峰」。
    pub(crate) jump_cut_armed: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            on_ground: false,
            inv: Inventory::default(),
            hp: PLAYER_MAX_HP,
            max_hp: PLAYER_MAX_HP,
            mp: PLAYER_MAX_MP,
            max_mp: PLAYER_MAX_MP,
            iframes: 0.0,
            pending_respawn: false,
            facing: 1.0,
            walk_phase: 0.0,
            swing_cd: 0.0,
            fall_start_y: None,
            mana_regen_cd: 0.0,
            home_spawn: None,
            grapple: None,
            extra_jumps: 0,
            just_landed: false,
            coyote_t: 0.0,
            jump_buffer_t: 0.0,
            drop_through_t: 0.0,
            jump_cut_armed: false,
        }
    }

    pub fn hurt(&mut self, dmg: f32) {
        self.hurt_hit(DamageHit::environmental(dmg));
    }

    pub fn hurt_hit(&mut self, hit: DamageHit) {
        if self.iframes > 0.0 || hit.amount <= 0.0 {
            return;
        }
        let typed = resolve_damage(hit, self.resist());
        let mitigated = typed * (1.0 - self.defense());
        if mitigated <= 0.0 {
            return;
        }
        self.hp = (self.hp - mitigated).max(0.0);
        self.iframes = 0.7;
        if self.hp <= 0.0 {
            self.pending_respawn = true;
        }
    }

    pub fn resist(&self) -> ResistProfile {
        if self.inv.has_armor() {
            ResistProfile::wood_armor()
        } else {
            ResistProfile::neutral()
        }
    }

    /// 已装备木甲则减伤（比例 0..=1，叠在矩阵之后）。
    pub fn defense(&self) -> f32 {
        if self.inv.has_armor() { 0.35 } else { 0.0 }
    }

    pub fn try_spend_mana(&mut self, cost: f32) -> bool {
        if cost <= 0.0 {
            return true;
        }
        if self.mp + 0.01 < cost {
            return false;
        }
        self.mp = (self.mp - cost).max(0.0);
        self.mana_regen_cd = 0.55;
        true
    }

    pub fn refill_vitals(&mut self) {
        self.hp = self.max_hp;
        self.mp = self.max_mp;
    }

    pub fn on_ladder(&self, world: &World) -> bool {
        let (px, py, pw, ph) = self.hitbox();
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let tx = wrap_tx((cx / TILE).floor() as i32);
        let ty = (cy / TILE).floor() as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if world.get(tx + dx, ty + dy).is_ladder() {
                    return true;
                }
            }
        }
        false
    }

    pub fn respawn_at(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.vx = 0.0;
        self.vy = 0.0;
        self.fall_start_y = None;
        self.grapple = None;
        self.refill_vitals();
        self.iframes = 1.5;
        self.pending_respawn = false;
    }

    pub fn mine_damage(&self) -> u16 {
        self.selected_item().mine_power().unwrap_or(HAND_DAMAGE)
    }

    pub fn can_swing(&self) -> bool {
        self.swing_cd <= 0.0
    }

    pub fn mark_swing(&mut self) {
        self.swing_cd = SWING_INTERVAL;
    }

    pub fn mark_swing_for(&mut self, interval: f32) {
        self.swing_cd = interval.max(0.08);
    }

    /// 挥击余辉强度：刚挥击时 ≈1，冷却结束 →0。
    pub fn swing_flash(&self) -> f32 {
        (self.swing_cd / SWING_INTERVAL).clamp(0.0, 1.0)
    }

    /// 当前手持物；空快捷栏格视为空手（`ItemId(0)`）。
    pub fn selected_item(&self) -> ItemId {
        self.inv.selected_item().unwrap_or(ItemId(0))
    }

    pub fn select_hotbar(&mut self, i: usize) {
        self.inv.select_hotbar(i);
    }

    pub fn select_item(&mut self, id: ItemId) {
        let _ = self.inv.select_first(id);
    }

    pub fn hitbox(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, HIT_W, HIT_H)
    }

    pub fn update(&mut self, world: &World, input: &Input, dt: f32) {
        movement::tick(self, world, input, dt);

        if self.on_ground && self.vx.abs() > 1.0 {
            self.walk_phase += self.vx.abs() * dt * 0.12;
        } else if !self.on_ground {
            self.walk_phase = 0.0;
        }
        if self.swing_cd > 0.0 {
            self.swing_cd -= dt;
        }
        if self.iframes > 0.0 {
            self.iframes -= dt;
        }
        if self.mana_regen_cd > 0.0 {
            self.mana_regen_cd -= dt;
        } else if self.mp < self.max_mp {
            self.mp = (self.mp + MANA_REGEN * dt).min(self.max_mp);
        }
    }

    pub(crate) fn move_axis(&mut self, world: &World, dx: f32, dy: f32) {
        let prev_x = self.x;
        let prev_y = self.y;
        self.x += dx;
        self.y += dy;
        if dx != 0.0 {
            self.x = wrap_xf(self.x);
        }
        if dy != 0.0 {
            // 水平步进不清落地标志；竖直步进时重算
            self.on_ground = false;
        }

        let min_tx = (self.x / TILE).floor() as i32;
        let span = if self.x + HIT_W > world_pixel_w() {
            ((self.x + HIT_W - world_pixel_w()) / TILE).floor() as i32 + 1
        } else {
            0
        };
        let max_tx = ((self.x + HIT_W) / TILE).floor() as i32 + span;
        let min_ty = (self.y / TILE).floor() as i32;
        let max_ty = ((self.y + HIT_H) / TILE).floor() as i32;

        for ty in min_ty..=max_ty {
            if ty < 0 || ty >= WORLD_H {
                continue;
            }
            for tx in min_tx..=max_tx {
                let block = world.get(tx, ty);
                if !Self::collides_motion(block, dx, dy, self.drop_through_t > 0.0) {
                    continue;
                }
                let bx = tile_x_near(tx, self.x + HIT_W * 0.5);
                let by = ty as f32 * TILE;
                if !aabb_overlap(self.x, self.y, HIT_W, HIT_H, bx, by, TILE, TILE) {
                    continue;
                }

                // 平台不参与水平阻挡与上台阶。
                if dx != 0.0 {
                    if block.is_platform() {
                        continue;
                    }
                    let step_y = by - HIT_H;
                    let lift = self.y - step_y;
                    if lift > 0.0 && lift <= TILE + 0.05 {
                        let old_y = self.y;
                        self.y = step_y;
                        if !Self::hitbox_blocked(world, self.x, self.y) {
                            self.on_ground = true;
                            continue;
                        }
                        self.y = old_y;
                    }
                    if dx > 0.0 {
                        self.x = wrap_xf(bx - HIT_W);
                        self.vx = 0.0;
                    } else {
                        self.x = wrap_xf(bx + TILE);
                        self.vx = 0.0;
                    }
                }

                if dy > 0.0 {
                    // 只认「从上落下」：上一帧脚底须在方块顶面之上。
                    // 否则侧面蹭树干会被当成落在树腰/树顶，再摔死。
                    let prev_foot = prev_y + HIT_H;
                    if block.is_platform() {
                        // 平台更严：脚须从顶面上方落下，且非下穿窗。
                        if self.drop_through_t > 0.0 || prev_foot > by + 0.05 {
                            continue;
                        }
                    } else if prev_foot > by + 0.05 {
                        continue;
                    }
                    self.y = by - HIT_H;
                    self.vy = 0.0;
                    self.on_ground = true;
                } else if dy < 0.0 {
                    if block.is_platform() {
                        continue;
                    }
                    let prev_head = prev_y;
                    let block_bottom = by + TILE;
                    // 只认头撞方块底；侧向重叠不把人吸进天花板。
                    if prev_head < block_bottom - 0.05 {
                        continue;
                    }
                    self.y = by + TILE;
                    self.vy = 0.0;
                }
            }
        }

        // 水平步进后若仍嵌在实心里，推回（竖直步已处理落地）。
        if dx != 0.0 && Self::hitbox_blocked(world, self.x, self.y) {
            self.x = prev_x;
            self.vx = 0.0;
        }

        self.y = self.y.clamp(0.0, WORLD_H as f32 * TILE - HIT_H);
    }

    /// 身体重叠检测：平台不挡身体（只挡自上而下的脚，见 `move_axis`）。
    fn hitbox_blocked(world: &World, x: f32, y: f32) -> bool {
        let min_tx = (x / TILE).floor() as i32;
        let span = if x + HIT_W > world_pixel_w() {
            ((x + HIT_W - world_pixel_w()) / TILE).floor() as i32 + 1
        } else {
            0
        };
        let max_tx = ((x + HIT_W) / TILE).floor() as i32 + span;
        let min_ty = (y / TILE).floor() as i32;
        let max_ty = ((y + HIT_H) / TILE).floor() as i32;
        for ty in min_ty..=max_ty {
            if ty < 0 || ty >= WORLD_H {
                continue;
            }
            for tx in min_tx..=max_tx {
                let block = world.get(tx, ty);
                if !block.blocks_motion() || block.is_platform() {
                    continue;
                }
                let bx = tile_x_near(tx, x + HIT_W * 0.5);
                let by = ty as f32 * TILE;
                if aabb_overlap(x, y, HIT_W, HIT_H, bx, by, TILE, TILE) {
                    return true;
                }
            }
        }
        false
    }

    /// 本步是否与该方块发生运动阻挡（平台仅下落且非下穿时）。
    fn collides_motion(block: BlockId, dx: f32, dy: f32, dropping: bool) -> bool {
        if !block.blocks_motion() {
            return false;
        }
        if block.is_platform() {
            return !dropping && dx == 0.0 && dy > 0.0;
        }
        true
    }

    /// 按住挖掘：有冷却的挥击，扣方块 HP；空格有墙则挖墙。打碎掉到地面。
    pub fn dig_hold(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        if self.swing_cd > 0.0 {
            return None;
        }
        if !world.in_bounds(tx, ty) {
            return None;
        }
        let id = world.get(tx, ty);
        if !id.mineable() {
            if id == BlockId::POD {
                self.swing_cd = SWING_INTERVAL;
                return Some("逃生舱太硬了".into());
            }
            // 前景可透视且有墙 → 挖墙
            if Self::can_mine_wall_through(id) && world.get_wall(tx, ty).mineable() {
                return self.dig_wall(world, tx, ty);
            }
            return None;
        }
        let cx = self.x + HIT_W * 0.5;
        let cy = self.y + HIT_H * 0.5;
        let bx = tile_x_near(tx, cx) + TILE * 0.5;
        let by = ty as f32 * TILE + TILE * 0.5;
        let dist = (wrap_delta_x(cx, bx).hypot(by - cy)) / TILE;
        if dist > 4.5 {
            return Some("太远了".into());
        }

        self.swing_cd = SWING_INTERVAL;
        if let Some(need) = id.mine_power_need() {
            let power = self.selected_item().mine_power().unwrap_or(0);
            if power < need {
                return Some(format!(
                    "{} 需要更强的镐（需 {need}，现有 {power}）",
                    id.label()
                ));
            }
        }
        let dmg = self.mine_damage();
        let tool = self.selected_item();
        if let Some(broken) = world.apply_damage(tx, ty, dmg) {
            Self::spawn_break_drops(world, tx, ty, broken);
            // 挖掉床则清除家园点（若指向此格）
            if broken == BlockId::BED {
                if let Some((hx, hy)) = self.home_spawn {
                    let bx = tile_x_near(tx, hx) + TILE * 0.5;
                    let by = ty as f32 * TILE + TILE * 0.5;
                    if (hx - bx).abs() < TILE && (hy - by).abs() < TILE {
                        self.home_spawn = None;
                    }
                }
            }
            let mut msg = if broken.drop_item().is_some() {
                format!("{} 掉落了", broken.label())
            } else {
                format!("{} 碎了", broken.label())
            };
            if let Some(broke) = self.inv.wear_tool(tool, 1) {
                msg = format!("{msg}；{broke}");
            }
            Some(msg)
        } else {
            let left = world.hp_left(tx, ty);
            if let Some(broke) = self.inv.wear_tool(tool, 1) {
                return Some(broke);
            }
            Some(format!("{} {}/{}", id.label(), left, id.max_hp()))
        }
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

    /// 锤类专用：拆背景墙。
    pub fn dig_wall_at(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        if self.swing_cd > 0.0 {
            return None;
        }
        if !world.in_bounds(tx, ty) {
            return None;
        }
        self.dig_wall(world, tx, ty)
    }

    fn dig_wall(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        let wall = world.get_wall(tx, ty);
        if !wall.mineable() {
            return None;
        }
        let cx = self.x + HIT_W * 0.5;
        let cy = self.y + HIT_H * 0.5;
        let bx = tile_x_near(tx, cx) + TILE * 0.5;
        let by = ty as f32 * TILE + TILE * 0.5;
        let dist = (wrap_delta_x(cx, bx).hypot(by - cy)) / TILE;
        if dist > 4.5 {
            return Some("太远了".into());
        }
        self.swing_cd = SWING_INTERVAL;
        // 墙伤害略低于前景，便于徒手清洞
        let dmg = (self.mine_damage() as f32 * 1.25).ceil() as u16;
        let tool = self.selected_item();
        if let Some(broken) = world.apply_wall_damage(tx, ty, dmg) {
            if let Some(item) = broken.drop_item() {
                world.spawn_drop_at_tile(tx, ty, item, 1);
            }
            let mut msg = format!("{} 掉落了", broken.label());
            if let Some(broke) = self.inv.wear_tool(tool, 1) {
                msg = format!("{msg}；{broke}");
            }
            Some(msg)
        } else {
            let left = world.wall_hp_left(tx, ty);
            if let Some(broke) = self.inv.wear_tool(tool, 1) {
                return Some(broke);
            }
            Some(format!("{} {}/{}", wall.name(), left, wall.max_hp()))
        }
    }

    fn spawn_break_drops(world: &mut World, tx: i32, ty: i32, broken: BlockId) {
        if let Some(item) = broken.drop_item() {
            world.spawn_drop_at_tile(tx, ty, item, 1);
        }
        // 砍木有概率掉树苗
        if broken == BlockId::WOOD {
            let roll = ((tx.wrapping_mul(31) ^ ty.wrapping_mul(17)) as u32) % 100;
            if roll < 35 {
                world.spawn_drop_at_tile(tx, ty, ItemId::SAPLING, 1);
            }
        }
    }

    /// 调试/demo：一次打满血打碎并生成掉落。
    pub fn dig_break(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        if !world.in_bounds(tx, ty) {
            return None;
        }
        let id = world.get(tx, ty);
        if !id.mineable() {
            return None;
        }
        let dmg = id.max_hp();
        if let Some(broken) = world.apply_damage(tx, ty, dmg) {
            Self::spawn_break_drops(world, tx, ty, broken);
            Some(broken.label())
        } else {
            None
        }
    }

    /// 走近地面掉落物拾取进背包。
    pub fn pickup_nearby(&mut self, world: &mut World) -> Option<String> {
        let (px, py, pw, ph) = self.hitbox();
        let got = world.try_pickup(px, py, pw, ph);
        if got.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for (item, n) in got {
            let left = self.inv.add(item, n);
            let took = n.saturating_sub(left);
            if took > 0 {
                parts.push(format!("{}×{}", item.label(), took));
            }
            if left > 0 {
                world.spawn_drop_at_tile(
                    wrap_tx(((self.x + HIT_W * 0.5) / TILE).floor() as i32),
                    ((self.y + HIT_H * 0.5) / TILE).floor() as i32,
                    item,
                    left,
                );
            }
        }
        if parts.is_empty() {
            return Some("背包已满".into());
        }
        Some(format!("拾取 {}", parts.join(" ")))
    }

    pub fn try_place(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        if !world.in_bounds(tx, ty) || world.get(tx, ty).solid() {
            return None;
        }
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
            return None;
        }
        let cx = self.x + HIT_W * 0.5;
        let cy = self.y + HIT_H * 0.5;
        let bx = tile_x_near(tx, cx);
        let by = ty as f32 * TILE;
        if (wrap_delta_x(cx, bx + TILE * 0.5).hypot(by + TILE * 0.5 - cy)) / TILE > 4.5 {
            return Some("太远了".into());
        }
        if aabb_overlap(self.x, self.y, HIT_W, HIT_H, bx, by, TILE, TILE) {
            // 树苗不挡人，仍可种脚下旁格；仅实心放置挡自己
            let item = self.selected_item();
            if item.as_block().map(|b| b.blocks_motion()).unwrap_or(true) {
                return Some("占着自己".into());
            }
        }

        let item = self.selected_item();
        if item == ItemId::SAPLING {
            if self.inv.get(ItemId::SAPLING) == 0 {
                return Some("没有树苗".into());
            }
            if !world.can_plant_sapling(tx, ty) {
                return Some("需种在草皮/泥土上且上方净空".into());
            }
            let _ = self.inv.try_take(ItemId::SAPLING, 1);
            world.plant_sapling(tx, ty);
            return Some("种下树苗".into());
        }

        let Some(block) = item.as_block() else {
            return Some("不可放置".into());
        };
        if self.inv.get(item) == 0 {
            return Some("没有方块".into());
        }
        let _ = self.inv.try_take(item, 1);
        world.set(tx, ty, block);
        if block == BlockId::BED {
            let bx = tile_x_near(tx, self.x) + TILE * 0.5;
            let by = ty as f32 * TILE;
            self.home_spawn = Some((bx, by - HIT_H));
            return Some("床（已设为家园重生点）".into());
        }
        Some(block.label())
    }

    /// 中键铺背景墙（泥土/石头）。
    pub fn try_place_wall(&mut self, world: &mut World, tx: i32, ty: i32) -> Option<String> {
        if !world.in_bounds(tx, ty) {
            return None;
        }
        let item = self.selected_item();
        let Some(wall) = item.as_wall() else {
            return Some("手持泥土/石头/木材可铺墙".into());
        };
        if self.inv.get(item) == 0 {
            return Some("没有材料".into());
        }
        let cx = self.x + HIT_W * 0.5;
        let cy = self.y + HIT_H * 0.5;
        let bx = tile_x_near(tx, cx) + TILE * 0.5;
        let by = ty as f32 * TILE + TILE * 0.5;
        if (wrap_delta_x(cx, bx).hypot(by - cy)) / TILE > 4.5 {
            return Some("太远了".into());
        }
        if world.get_wall(tx, ty) == wall {
            return Some("已有同款墙".into());
        }
        let _ = self.inv.try_take(item, 1);
        world.set_wall(tx, ty, wall);
        Some(wall.label())
    }

    /// F：食用当前选中食物。
    pub fn try_eat(&mut self) -> Option<String> {
        let item = self.selected_item();
        let Some(heal) = item.heal_amount() else {
            return Some("不可食用".into());
        };
        if self.inv.get(item) == 0 {
            return Some(format!("没有{}", item.label()));
        }
        if self.hp >= self.max_hp - 0.5 {
            return Some("生命已满".into());
        }
        let _ = self.inv.try_take(item, 1);
        self.hp = (self.hp + heal).min(self.max_hp);
        Some(format!("食用{} +{heal:.0} HP", item.label()))
    }

    /// 尝试制作：`recipe_idx` 为 `RECIPES` 下标。
    pub fn try_craft(&mut self, world: &World, recipe_idx: usize) -> Option<String> {
        let recipe = RECIPES.get(recipe_idx)?;
        let (px, py, pw, ph) = self.hitbox();
        let at_wb = world.near_workbench(px, py, pw, ph);
        match recipe.station {
            CraftStation::Hand => {}
            CraftStation::Workbench if !at_wb => {
                return Some("需要靠近工作台".into());
            }
            CraftStation::Workbench => {}
            CraftStation::Furnace if !world.near_furnace(px, py, pw, ph) => {
                return Some("需要靠近熔炉".into());
            }
            CraftStation::Furnace => {}
        }
        if !self.inv.can_pay(recipe.inputs) {
            return Some(format!("材料不足：{}", recipe.label));
        }
        if !self.inv.pay(recipe.inputs) {
            return Some("材料不足".into());
        }
        self.inv.add(recipe.output, recipe.output_count);
        if recipe.output == ItemId::WOOD_ARMOR && self.inv.armor.is_none() {
            let _ = self.inv.equip_armor(ItemId::WOOD_ARMOR);
        }
        Some(format!("制成 {}", recipe.label))
    }

    pub fn craft_station_now(&self, world: &World) -> CraftStation {
        let (px, py, pw, ph) = self.hitbox();
        if world.near_furnace(px, py, pw, ph) {
            CraftStation::Furnace
        } else if world.near_workbench(px, py, pw, ph) {
            CraftStation::Workbench
        } else {
            CraftStation::Hand
        }
    }

    pub fn interact_pod(&self, world: &World) -> Option<&'static str> {
        let tx = wrap_tx(((self.x + HIT_W * 0.5) / TILE).floor() as i32);
        let ty = ((self.y + HIT_H * 0.5) / TILE).floor() as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if world.get(tx + dx, ty + dy) == BlockId::POD {
                    return Some("逃生舱：白日按 E 余温，入夜按 E 休息。");
                }
            }
        }
        None
    }
}

fn aabb_overlap(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}
