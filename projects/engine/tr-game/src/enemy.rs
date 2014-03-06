//! 地表敌怪：史莱姆、夜间僵尸与恶魔眼（`NPC_1` / `NPC_3` / `NPC_2`）。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{DamageHit, ItemId, ResistProfile, resolve_damage};

use crate::player::Player;
use crate::world::{
    TILE, WORLD_H, WORLD_W, World, screen_len, screen_of, tile_x_near, world_pixel_w, wrap_delta_x,
    wrap_tx,
};

const GRAVITY: f32 = TILE * 40.0;
const SLIME_W: f32 = TILE * 0.9;
const SLIME_H: f32 = TILE * 0.7;
const ZOMBIE_W: f32 = TILE * (18.0 / 16.0);
const ZOMBIE_H: f32 = TILE * (40.0 / 16.0);
const EYE_W: f32 = TILE * 1.5;
const EYE_H: f32 = TILE * 0.85;
const TOUCH_CD: f32 = 0.85;
const WINDUP_TIME: f32 = 0.32;

/// 敌怪身份（AI / 视觉 / 抗性绑定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// 蓝史莱姆：`AI Style 1`。
    BlueSlime,
    /// 僵尸：`AI Style 3` Fighter。
    Zombie,
    /// 恶魔眼：`AI Style 2`。
    DemonEye,
}

impl EnemyKind {
    fn label(self) -> &'static str {
        match self {
            Self::BlueSlime => "蓝史莱姆",
            Self::Zombie => "僵尸",
            Self::DemonEye => "恶魔眼",
        }
    }

    fn body(self) -> Color {
        match self {
            Self::BlueSlime => Color::rgb(0.35, 0.55, 0.95),
            Self::Zombie => Color::rgb(0.45, 0.55, 0.35),
            Self::DemonEye => Color::rgb(0.85, 0.25, 0.35),
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::BlueSlime => Color::rgb(0.55, 0.75, 1.0),
            Self::Zombie => Color::rgb(0.65, 0.75, 0.45),
            Self::DemonEye => Color::rgb(0.95, 0.45, 0.4),
        }
    }

    fn core(self) -> Color {
        match self {
            Self::BlueSlime => Color::rgb(0.85, 0.92, 1.0),
            Self::Zombie => Color::rgb(0.9, 0.85, 0.7),
            Self::DemonEye => Color::rgb(1.0, 0.85, 0.4),
        }
    }

    fn resist(self) -> ResistProfile {
        match self {
            Self::BlueSlime => ResistProfile::slime(),
            Self::Zombie | Self::DemonEye => ResistProfile::neutral(),
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::BlueSlime => SLIME_W,
            Self::Zombie => ZOMBIE_W,
            Self::DemonEye => EYE_W,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::BlueSlime => SLIME_H,
            Self::Zombie => ZOMBIE_H,
            Self::DemonEye => EYE_H,
        }
    }

    /// Classic 接触伤（待正版回放复核）。
    fn touch_damage(self) -> f32 {
        match self {
            Self::BlueSlime => 7.0,
            Self::Zombie => 14.0,
            Self::DemonEye => 18.0,
        }
    }

    fn flies(self) -> bool {
        matches!(self, Self::DemonEye)
    }

    fn max_hp(self) -> f32 {
        match self {
            Self::BlueSlime => 25.0,
            Self::Zombie => 45.0,
            Self::DemonEye => 60.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub hurt_cd: f32,
    pub touch_cd: f32,
    pub facing: f32,
    pub resist: ResistProfile,
    /// 弹跳 / 飞行动画相位。
    pub bob: f32,
    /// 接触攻击前摇剩余时间。
    pub windup_t: f32,
    /// 史莱姆：受伤或入夜后才追玩家。
    pub aggro: bool,
}

impl Enemy {
    pub fn slime(x: f32, y: f32) -> Self {
        let kind = EnemyKind::BlueSlime;
        let hp = kind.max_hp();
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp,
            max_hp: hp,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
            aggro: false,
        }
    }

    pub fn zombie(x: f32, y: f32) -> Self {
        let kind = EnemyKind::Zombie;
        let hp = kind.max_hp();
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp,
            max_hp: hp,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
            aggro: true,
        }
    }

    pub fn demon_eye(x: f32, y: f32) -> Self {
        let kind = EnemyKind::DemonEye;
        let hp = kind.max_hp();
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp,
            max_hp: hp,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
            aggro: true,
        }
    }

    /// 经抗性矩阵结算伤害；`knock_dir` 为水平击退符号（相对 TILE）。
    pub fn apply_hit(&mut self, hit: DamageHit, knock_dir: f32) -> f32 {
        let actual = resolve_damage(hit, self.resist);
        if actual <= 0.0 {
            return 0.0;
        }
        self.hp -= actual;
        self.hurt_cd = 0.35;
        self.aggro = true;
        self.vx = knock_dir.signum() * TILE * hit.amount.max(1.0).min(14.0);
        if knock_dir.abs() < 0.01 {
            self.vx = self.facing * TILE * 8.0;
        }
        actual
    }

    pub fn hitbox(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.kind.width(), self.kind.height())
    }

    /// 击杀掉落：史莱姆凝胶+铜币，僵尸铜币。
    pub fn drop_loot(
        &self,
        world: &mut World,
        dust: &mut Vec<crate::fx::DustParticle>,
        ecx: f32,
        ecy: f32,
        tx: i32,
        ty: i32,
    ) {
        match self.kind {
            EnemyKind::BlueSlime => {
                world.spawn_drop_at_tile(tx, ty, ItemId::GEL, 2);
                let coins = 1 + ((world.seed ^ (tx as u64).wrapping_mul(17)) % 3) as u32;
                world.spawn_drop_at_tile(tx, ty, ItemId::COPPER_COIN, coins);
                crate::fx::burst_gel(dust, ecx, ecy);
            }
            EnemyKind::Zombie => {
                let coins = 3 + ((world.seed ^ (tx as u64).wrapping_mul(31)) % 5) as u32;
                world.spawn_drop_at_tile(tx, ty, ItemId::COPPER_COIN, coins);
                crate::fx::burst_dust(dust, ecx, ecy, 0.7);
            }
            EnemyKind::DemonEye => {
                let coins = 2 + ((world.seed ^ (tx as u64).wrapping_mul(23)) % 4) as u32;
                world.spawn_drop_at_tile(tx, ty, ItemId::COPPER_COIN, coins);
                crate::fx::burst_dust(dust, ecx, ecy, 0.45);
            }
        }
    }

    pub fn draw(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        slime: Option<crate::tiles::TileView>,
        slime_frames: u32,
        zombie: Option<crate::npc::NpcView>,
        zombie_cell: (u32, u32),
        eye: Option<crate::npc::NpcView>,
        eye_cell: (u32, u32),
    ) {
        match self.kind {
            EnemyKind::Zombie => self.draw_humanoid(draw, cam_x, cam_y, zombie, zombie_cell),
            EnemyKind::DemonEye => self.draw_eye(draw, cam_x, cam_y, eye, eye_cell),
            EnemyKind::BlueSlime => self.draw_slime(draw, cam_x, cam_y, slime, slime_frames),
        }
    }

    fn screen_x(&self, cam_x: f32) -> f32 {
        screen_of(self.x, cam_x)
    }

    fn draw_hp_bar(&self, draw: &mut DrawList, sx: f32, sy: f32, bw: f32) {
        // 正版仅在受伤后短暂显示；不常驻血条。
        if self.hurt_cd <= 0.0 {
            return;
        }
        let ratio = (self.hp / self.max_hp).clamp(0.0, 1.0);
        draw.fill_rect(
            Rect::new(sx, sy - 6.0, bw, 3.0),
            Color::rgb(0.12, 0.08, 0.14),
        );
        draw.fill_rect(
            Rect::new(sx, sy - 6.0, bw * ratio, 3.0),
            Color::rgb(0.85, 0.28, 0.42),
        );
    }

    fn draw_humanoid(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        view: Option<crate::npc::NpcView>,
        cell: (u32, u32),
    ) {
        let bw = screen_len(self.kind.width());
        let bh = screen_len(self.kind.height());
        let sx = self.screen_x(cam_x);
        let sy = screen_of(self.y, cam_y);
        let flash = self.hurt_cd > 0.0 || self.windup_t > 0.0;
        if let Some(view) = view {
            let mut uv = view.uv;
            // 行走时在竖条帧间切换（`npcFrameCount`）。
            let frames = crate::sheets::npc_frame_count(tr_core::NpcId::ZOMBIE).max(1);
            if frames > 1 {
                let fh = uv.h;
                let frame = if self.vx.abs() > 1.0 {
                    (((self.bob * 3.0).floor() as i32).rem_euclid(frames as i32)) as u32
                } else {
                    0
                };
                uv.y = frame as f32 * fh;
                uv.h = fh;
            }
            if self.facing < 0.0 {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            let sprite_w = screen_len(cell.0 as f32);
            let sprite_h = screen_len(cell.1 as f32);
            let ox = sx + bw * 0.5 - sprite_w * 0.5;
            let oy = sy + bh - sprite_h;
            let tint = if flash {
                Color::rgb(1.0, 0.72, 0.72)
            } else {
                Color::rgba(1.0, 1.0, 1.0, 1.0)
            };
            draw.tex_rect(view.tex, Rect::new(ox, oy, sprite_w, sprite_h), uv, tint);
        } else {
            let body = if flash {
                Color::rgb(0.95, 0.55, 0.55)
            } else {
                self.kind.body()
            };
            draw.fill_rect(Rect::new(sx, sy, bw, bh), body);
        }
        self.draw_hp_bar(draw, sx, sy, bw);
    }

    fn draw_eye(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        view: Option<crate::npc::NpcView>,
        cell: (u32, u32),
    ) {
        let bw = screen_len(self.kind.width());
        let bh = screen_len(self.kind.height());
        let sx = self.screen_x(cam_x);
        let bob = screen_len(self.bob.sin() * 6.0);
        let sy = screen_of(self.y, cam_y) + bob;
        let flash = self.hurt_cd > 0.0 || self.windup_t > 0.0;
        if let Some(view) = view {
            let mut uv = view.uv;
            let frames = crate::sheets::npc_frame_count(tr_core::NpcId::DEMON_EYE).max(1);
            if frames > 1 {
                let fh = uv.h;
                let frame = if self.bob.sin() > 0.0 { 0u32 } else { 1.min(frames - 1) };
                uv.y = frame as f32 * fh;
                uv.h = fh;
            }
            if self.facing < 0.0 {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            let sprite_w = screen_len(cell.0 as f32);
            let sprite_h = screen_len(cell.1 as f32);
            let ox = sx + bw * 0.5 - sprite_w * 0.5;
            let oy = sy + bh * 0.5 - sprite_h * 0.5;
            let tint = if flash {
                Color::rgb(1.0, 0.65, 0.65)
            } else {
                // `NPC_2` 已是着色图，不再乘身份色。
                Color::rgba(1.0, 1.0, 1.0, 1.0)
            };
            draw.tex_rect(view.tex, Rect::new(ox, oy, sprite_w, sprite_h), uv, tint);
        } else {
            let body = if flash {
                Color::rgb(0.95, 0.45, 0.45)
            } else {
                self.kind.body()
            };
            draw.fill_rect(Rect::new(sx, sy, bw, bh), body);
        }
        self.draw_hp_bar(draw, sx, sy, bw);
    }

    fn draw_slime(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        slime: Option<crate::tiles::TileView>,
        slime_frames: u32,
    ) {
        let sx = self.screen_x(cam_x);
        let sy = screen_of(self.y, cam_y);
        let flash = self.hurt_cd > 0.0 || self.windup_t > 0.0;
        let hit_w = screen_len(SLIME_W);
        let hit_h = screen_len(SLIME_H);

        if let Some(view) = slime {
            let frames = slime_frames.max(1);
            let mut uv = view.uv;
            let fh = uv.h;
            // 弹跳相位切帧（`npcFrameCount[BlueSlime]=2`）。
            let frame = if self.bob.sin() > 0.25 {
                1u32.min(frames - 1)
            } else {
                0
            };
            uv.y = frame as f32 * fh;
            uv.h = fh;
            // 按贴图像素尺寸绘制，不拉伸进碰撞盒。
            let cell_w = 32.0;
            let cell_h = 26.0;
            let sprite_w = screen_len(cell_w);
            let sprite_h = screen_len(cell_h);
            let ox = sx + hit_w * 0.5 - sprite_w * 0.5;
            let oy = sy + hit_h - sprite_h;
            let body = self.kind.body();
            let tint = if flash {
                Color::rgb(1.0, 0.72, 0.88)
            } else {
                // `NPC_1` 为灰度，乘蓝史莱姆色。
                Color::rgba(body.r, body.g, body.b, 1.0)
            };
            if self.facing < 0.0 {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            draw.tex_rect(view.tex, Rect::new(ox, oy, sprite_w, sprite_h), uv, tint);
            self.draw_hp_bar(draw, ox, oy, sprite_w);
            return;
        }

        let body = if flash {
            Color::rgb(0.95, 0.55, 0.75)
        } else {
            self.kind.body()
        };
        draw.fill_rect(Rect::new(sx, sy, hit_w, hit_h), body);
        self.draw_hp_bar(draw, sx, sy, hit_w);
    }
}


fn rim_pre(enemy: &Enemy, flash: bool) -> Color {
    if flash {
        Color::rgb(1.0, 0.85, 0.95)
    } else {
        enemy.kind.rim()
    }
}

/// 在地表远离出生点刷几只凝胶。
pub fn spawn_surface_slimes(world: &World, out: &mut Vec<Enemy>) {
    out.clear();
    let spawn_tx = crate::world::SPAWN_TX;
    let mut x = 20;
    while x < WORLD_W - 4 {
        let tx = wrap_tx(x);
        let near_spawn = (tx - spawn_tx).abs() < 24;
        if !near_spawn && (tx as u64 * 17 + world.seed) % 11 == 0 {
            let sh = world.surface_at(tx);
            let px = tx as f32 * TILE + 2.0;
            let py = sh as f32 * TILE - SLIME_H;
            out.push(Enemy::slime(px, py));
            x += 14;
        } else {
            x += 3;
        }
    }
    if out.is_empty() {
        let a = wrap_tx(spawn_tx + 40);
        let b = wrap_tx(spawn_tx - 40);
        let sh = world.surface_at(a);
        out.push(Enemy::slime(a as f32 * TILE, sh as f32 * TILE - SLIME_H));
        let sh2 = world.surface_at(b);
        out.push(Enemy::slime(b as f32 * TILE, sh2 as f32 * TILE - SLIME_H));
    }
}

pub fn update_enemies(
    world: &World,
    enemies: &mut Vec<Enemy>,
    player: &mut Player,
    dt: f32,
    night: f32,
    sfx: &crate::sfx::SfxBank,
    audio: &spark_audio::AudioBus,
) {
    let (px, py, pw, ph) = player.hitbox();
    let pcx = px + pw * 0.5;
    let aggro = 1.0 + night * 0.85;
    let dmg_mul = 1.0 + night * 0.5;
    enemies.retain_mut(|e| {
        if e.hurt_cd > 0.0 {
            e.hurt_cd -= dt;
        }
        if e.touch_cd > 0.0 {
            e.touch_cd -= dt;
        }
        e.bob += dt * (6.0 + night * 2.0);
        if e.kind.flies() && night < 0.28 {
            // 日出灼烧：恶魔眼不会在白天停留。
            e.hp -= dt * 24.0;
        }

        let ew = e.kind.width();
        let eh = e.kind.height();
        let ecx = e.x + ew * 0.5;
        let dx = wrap_delta_x(ecx, pcx);
        e.facing = if dx.abs() > 4.0 {
            dx.signum()
        } else {
            e.facing
        };
        match e.kind {
            EnemyKind::DemonEye => {
                // AI Style 2：夜间飞向玩家，白天灼烧已在上方处理。
                let ecy = e.y + eh * 0.5;
                let pcy = py + ph * 0.5;
                let dy = pcy - ecy;
                let engage = dx.abs() < TILE * 36.0;
                let speed = if engage {
                    TILE * 3.8 * aggro
                } else {
                    TILE * 1.6
                };
                e.vx = speed * e.facing;
                e.vy = if engage {
                    (dy.signum() * TILE * 2.2 + (e.bob * 2.1).sin() * TILE * 0.25)
                        .clamp(-TILE * 4.0, TILE * 4.0)
                } else {
                    e.bob.sin() * TILE * 0.4
                };
            }
            EnemyKind::Zombie => {
                // AI Style 3 Fighter：只在夜间追玩家，白天灼烧消亡。
                if night < 0.35 {
                    e.hp -= dt * 40.0;
                }
                let chase = night > 0.35;
                let mut speed = if chase {
                    TILE * (1.8 + night * 1.0) * e.facing
                } else {
                    TILE * 0.6 * e.facing
                };
                if e.windup_t > 0.0 {
                    speed *= 0.15;
                }
                e.vx = speed;
                e.vy += GRAVITY * dt;
            }
            EnemyKind::BlueSlime => {
                // AI Style 1：无 aggro 时原地小跳；受伤或入夜后朝玩家跳。
                if night > 0.55 {
                    e.aggro = true;
                }
                e.vy += GRAVITY * dt;
                let on_ground = e.vy == 0.0
                    || world
                        .get(
                            wrap_tx(((e.x + ew * 0.5) / TILE).floor() as i32),
                            ((e.y + eh + 1.0) / TILE).floor() as i32,
                        )
                        .blocks_motion();
                if on_ground && e.vy >= 0.0 {
                    if e.aggro {
                        let hop = if dx.abs() < TILE * 18.0 {
                            TILE * 7.2
                        } else {
                            TILE * 5.2
                        };
                        e.vy = -hop;
                        e.vx = e.facing * TILE * 2.6;
                    } else {
                        // 闲逛：低跳 + 偶发换向。
                        e.vy = -TILE * 3.2;
                        if (e.bob * 0.37).sin().abs() < 0.08 {
                            e.facing = -e.facing;
                        }
                        e.vx = e.facing * TILE * 1.1;
                    }
                    if e.windup_t > 0.0 {
                        e.vx *= 0.2;
                        e.vy *= 0.55;
                    }
                }
            }
        }

        move_enemy(world, e, e.vx * dt, 0.0);
        move_enemy(world, e, 0.0, e.vy * dt);

        let (ex, ey, ew, eh) = e.hitbox();
        let touching = aabb(px, py, pw, ph, ex, ey, ew, eh);
        if touching && e.touch_cd <= 0.0 {
            if e.windup_t <= 0.0 {
                e.windup_t = WINDUP_TIME;
            } else {
                e.windup_t -= dt;
                if e.windup_t <= 0.0 {
                    player.hurt_hit(DamageHit::kinetic(e.kind.touch_damage() * dmg_mul));
                    crate::sfx::player_hurt(sfx, audio);
                    e.touch_cd = (TOUCH_CD * (1.0 - night * 0.25)).max(0.45);
                    e.windup_t = 0.0;
                    player.vx = -e.facing * TILE * (8.0 + night * 4.0);
                }
            }
        } else {
            e.windup_t = 0.0;
        }

        e.hp > 0.0
    });
}

fn move_enemy(world: &World, e: &mut Enemy, dx: f32, dy: f32) {
    let ew = e.kind.width();
    let eh = e.kind.height();
    e.x += dx;
    e.y += dy;
    if dx != 0.0 {
        e.x = e.x.clamp(0.0, world_pixel_w() - ew);
    }
    let min_tx = (e.x / TILE).floor() as i32 - 1;
    let max_tx = ((e.x + ew) / TILE).floor() as i32 + 1;
    let min_ty = (e.y / TILE).floor() as i32;
    let max_ty = ((e.y + eh) / TILE).floor() as i32;
    for ty in min_ty..=max_ty {
        if ty < 0 || ty >= WORLD_H {
            continue;
        }
        for tx in min_tx..=max_tx {
            if !world.get(tx, ty).blocks_motion() {
                continue;
            }
            let bx = tile_x_near(tx, e.x + ew * 0.5);
            let by = ty as f32 * TILE;
            if !aabb(e.x, e.y, ew, eh, bx, by, TILE, TILE) {
                continue;
            }
            if dx > 0.0 {
                e.x = (bx - ew).clamp(0.0, world_pixel_w() - ew);
                e.vx = 0.0;
                e.facing = -1.0;
            } else if dx < 0.0 {
                e.x = (bx + TILE).clamp(0.0, world_pixel_w() - ew);
                e.vx = 0.0;
                e.facing = 1.0;
            }
            if dy > 0.0 {
                e.y = by - eh;
                e.vy = 0.0;
            } else if dy < 0.0 {
                e.y = by + TILE;
                e.vy = 0.0;
            }
        }
    }
    e.y = e.y.clamp(0.0, WORLD_H as f32 * TILE - eh);
}

fn aabb(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

/// 玩家近战命中：返回击杀掉落描述。
pub fn try_melee(
    player: &Player,
    enemies: &mut Vec<Enemy>,
    world: &mut World,
    floaters: &mut Vec<crate::fx::DamageFloater>,
    dust: &mut Vec<crate::fx::DustParticle>,
) -> Option<String> {
    let wpn = player
        .selected_item()
        .weapon()
        .filter(|w| matches!(w.kind, tr_core::WeaponKind::Melee))?;
    let reach = TILE * wpn.reach_or_speed;
    let hit = DamageHit::new(wpn.damage, wpn.dtype);
    let (px, py, pw, ph) = player.hitbox();
    let pcx = px + pw * 0.5;
    let pcy = py + ph * 0.5;
    let mut hit_any = false;
    let mut killed = None;
    let mut last_actual = 0.0;
    for e in enemies.iter_mut() {
        if e.hurt_cd > 0.0 {
            continue;
        }
        let (ex, ey, ew, eh) = e.hitbox();
        let ecx = ex + ew * 0.5;
        let ecy = ey + eh * 0.5;
        let dx = wrap_delta_x(pcx, ecx);
        // 须朝向目标大致一侧
        if dx.signum() != 0.0 && dx.signum() != player.facing.signum() && dx.abs() > TILE * 0.4 {
            continue;
        }
        let dy = ecy - pcy;
        if dx.hypot(dy) > reach {
            continue;
        }
        let actual = e.apply_hit(hit, player.facing * wpn.knockback);
        crate::fx::push_hit(floaters, ecx, ey, actual);
        last_actual = actual;
        hit_any = true;
        if e.hp <= 0.0 {
            let tx = wrap_tx((ecx / TILE).floor() as i32);
            let ty = (ecy / TILE).floor() as i32;
            e.drop_loot(world, dust, ecx, ecy, tx, ty);
            killed = Some(format!(
                "{}碎裂（{} −{actual:.0}）",
                e.kind.label(),
                wpn.dtype.name()
            ));
        }
        break;
    }
    if killed.is_some() {
        enemies.retain(|e| e.hp > 0.0);
        killed
    } else if hit_any {
        Some(format!("{} −{last_actual:.0}", wpn.dtype.name()))
    } else {
        None
    }
}
