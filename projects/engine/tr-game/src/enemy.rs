//! 地表敌怪：史莱姆、夜间僵尸与恶魔眼（正版 `NPC_1` / `NPC_3` / `NPC_2`）。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{DamageHit, ItemId, ResistProfile, resolve_damage};

use crate::player::Player;
use crate::world::{
    TILE, WORLD_H, WORLD_W, World, tile_x_near, world_pixel_w, wrap_delta_x, wrap_tx,
};

const GRAVITY: f32 = TILE * 40.0;
const SLIME_W: f32 = TILE * 0.9;
const SLIME_H: f32 = TILE * 0.7;
const ZOMBIE_W: f32 = TILE * (20.0 / 16.0);
const ZOMBIE_H: f32 = TILE * (42.0 / 16.0);
const EYE_W: f32 = TILE * 1.5;
const EYE_H: f32 = TILE * 0.85;
const TOUCH_DAMAGE: f32 = 8.0;
const ZOMBIE_TOUCH: f32 = 12.0;
const EYE_TOUCH: f32 = 7.0;
const TOUCH_CD: f32 = 0.85;
const WINDUP_TIME: f32 = 0.32;

/// 敌怪身份（视觉与抗性绑定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// 蓝史莱姆：地表软体。
    ShadowSlime,
    /// 僵尸：夜间人形敌怪。
    Zombie,
    /// 恶魔眼：夜间飞行。白天会被阳光灼烧。
    DemonEye,
}

impl EnemyKind {
    fn label(self) -> &'static str {
        match self {
            Self::ShadowSlime => "蓝史莱姆",
            Self::Zombie => "僵尸",
            Self::DemonEye => "恶魔眼",
        }
    }

    fn body(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.35, 0.55, 0.95),
            Self::Zombie => Color::rgb(0.45, 0.55, 0.35),
            Self::DemonEye => Color::rgb(0.85, 0.25, 0.35),
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.55, 0.75, 1.0),
            Self::Zombie => Color::rgb(0.65, 0.75, 0.45),
            Self::DemonEye => Color::rgb(0.95, 0.45, 0.4),
        }
    }

    fn core(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.85, 0.92, 1.0),
            Self::Zombie => Color::rgb(0.9, 0.85, 0.7),
            Self::DemonEye => Color::rgb(1.0, 0.85, 0.4),
        }
    }

    fn resist(self) -> ResistProfile {
        match self {
            Self::ShadowSlime => ResistProfile::slime(),
            Self::Zombie => ResistProfile::neutral(),
            Self::DemonEye => ResistProfile::neutral(),
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::ShadowSlime => SLIME_W,
            Self::Zombie => ZOMBIE_W,
            Self::DemonEye => EYE_W,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::ShadowSlime => SLIME_H,
            Self::Zombie => ZOMBIE_H,
            Self::DemonEye => EYE_H,
        }
    }

    fn touch_damage(self) -> f32 {
        match self {
            Self::ShadowSlime => TOUCH_DAMAGE,
            Self::Zombie => ZOMBIE_TOUCH,
            Self::DemonEye => EYE_TOUCH,
        }
    }

    fn flies(self) -> bool {
        matches!(self, Self::DemonEye)
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
    /// 弹跳相位：驱动压扁/拉长剪影。
    pub bob: f32,
    /// 接触攻击前摇剩余时间；>0 时减速并闪轮廓。
    pub windup_t: f32,
}

impl Enemy {
    pub fn slime(x: f32, y: f32) -> Self {
        let kind = EnemyKind::ShadowSlime;
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: 36.0,
            max_hp: 36.0,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
        }
    }

    pub fn zombie(x: f32, y: f32) -> Self {
        let kind = EnemyKind::Zombie;
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: 55.0,
            max_hp: 55.0,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
        }
    }

    pub fn demon_eye(x: f32, y: f32) -> Self {
        let kind = EnemyKind::DemonEye;
        Self {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: 32.0,
            max_hp: 32.0,
            hurt_cd: 0.0,
            touch_cd: 0.0,
            facing: 1.0,
            resist: kind.resist(),
            bob: 0.0,
            windup_t: 0.0,
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
        self.vx = knock_dir.signum() * TILE * hit.amount.max(1.0).min(14.0);
        if knock_dir.abs() < 0.01 {
            // 无明确方向时沿当前朝向反弹
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
            EnemyKind::ShadowSlime => {
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
        zombie: Option<crate::npc::NpcView>,
        zombie_cell: (u32, u32),
        eye: Option<crate::npc::NpcView>,
        eye_cell: (u32, u32),
    ) {
        match self.kind {
            EnemyKind::Zombie => self.draw_zombie(draw, cam_x, cam_y, zombie, zombie_cell),
            EnemyKind::DemonEye => self.draw_zombie(draw, cam_x, cam_y, eye, eye_cell),
            EnemyKind::ShadowSlime => self.draw_slime(draw, cam_x, cam_y, slime),
        }
    }

    fn screen_x(&self, cam_x: f32) -> f32 {
        let sx = self.x - cam_x;
        let w = world_pixel_w();
        let mut d = sx;
        if d > w * 0.5 {
            d -= w;
        } else if d < -w * 0.5 {
            d += w;
        }
        d
    }

    fn draw_zombie(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        view: Option<crate::npc::NpcView>,
        cell: (u32, u32),
    ) {
        let bw = self.kind.width();
        let bh = self.kind.height();
        let sx = self.screen_x(cam_x);
        let bob = if self.kind.flies() {
            self.bob.sin() * 6.0
        } else {
            0.0
        };
        let sy = self.y - cam_y + bob;
        let flash = self.hurt_cd > 0.0 || self.windup_t > 0.0;
        if let Some(view) = view {
            let mut uv = view.uv;
            if self.facing < 0.0 {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            let sprite_w = cell.0 as f32 * (TILE / 16.0);
            let sprite_h = cell.1 as f32 * (TILE / 16.0);
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
            draw.fill_rect(
                Rect::new(sx + 4.0, sy + 4.0, bw - 8.0, 10.0),
                self.kind.core(),
            );
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

    fn draw_slime(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        slime: Option<crate::tiles::TileView>,
    ) {
        let sx = self.screen_x(cam_x);
        let sy = self.y - cam_y;
        let flash = self.hurt_cd > 0.0 || self.windup_t > 0.0;
        let squash = 1.0 + 0.12 * self.bob.sin();
        let stretch = 1.0 - 0.10 * self.bob.sin();
        let bw = SLIME_W * squash;
        let bh = SLIME_H * stretch;
        let ox = sx + (SLIME_W - bw) * 0.5;
        let oy = sy + (SLIME_H - bh);
        let foot_x = sx + SLIME_W * 0.5;
        let foot_y = sy + SLIME_H;
        // 前摇优先：风红脉动；否则追击速度脉动。
        let chase = (self.vx.abs() / (TILE * 4.0)).clamp(0.0, 1.0);
        let telegraph = if self.windup_t > 0.0 {
            0.85 + 0.15 * (self.bob * 8.0).sin().abs()
        } else {
            chase * (0.55 + 0.45 * (self.bob * 2.4).sin().abs())
        };

        // 触地椭圆阴影（三层软边）。
        for layer in 0..3 {
            let t = layer as f32 / 2.0;
            let a = 0.38 * (1.0 - t * 0.4);
            let rw = SLIME_W * (0.38 + t * 0.18);
            let rh = 2.2 + t * 1.4;
            let color = Color::rgba(0.02, 0.02, 0.06, a);
            let y0 = (foot_y - rh).floor() as i32;
            let y1 = (foot_y + rh * 0.3).ceil() as i32;
            for y in y0..=y1 {
                let dy = (y as f32 + 0.5 - foot_y) / rh.max(0.5);
                let inner = 1.0 - dy * dy;
                if inner <= 0.0 {
                    continue;
                }
                let half = rw * inner.sqrt();
                draw.fill_rect(Rect::new(foot_x - half, y as f32, half * 2.0, 1.0), color);
            }
        }

        if telegraph > 0.15 {
            let pulse = 1.0 + telegraph * 0.15;
            let tw = bw * pulse;
            let th = bh * pulse;
            draw.fill_rect(
                Rect::new(
                    ox + (bw - tw) * 0.5 - 2.0,
                    oy + (bh - th) - 2.0,
                    tw + 4.0,
                    th + 4.0,
                ),
                Color::rgba(
                    rim_pre(self, flash).r,
                    rim_pre(self, flash).g,
                    rim_pre(self, flash).b,
                    0.18 + 0.28 * telegraph,
                ),
            );
        }

        let body = if flash {
            Color::rgb(0.95, 0.55, 0.75)
        } else {
            self.kind.body()
        };
        let rim = rim_pre(self, flash);
        let dest = Rect::new(ox, oy, bw, bh);
        if let Some(view) = slime {
            let tint = if flash {
                Color::rgb(1.0, 0.72, 0.88)
            } else {
                Color::rgba(1.0, 1.0, 1.0, 1.0)
            };
            draw.tex_rect(view.tex, dest, view.uv, tint);
        } else {
            // 外轮廓描边（身份色）
            draw.fill_rect(
                Rect::new(ox - 1.0, oy - 1.0, bw + 2.0, bh + 2.0),
                Color::rgba(rim.r, rim.g, rim.b, 0.55),
            );
            draw.fill_rect(dest, body);
            // 高光核
            draw.fill_rect(
                Rect::new(ox + bw * 0.18, oy + bh * 0.18, bw * 0.28, bh * 0.28),
                Color::rgba(
                    self.kind.core().r,
                    self.kind.core().g,
                    self.kind.core().b,
                    0.55,
                ),
            );
        }
        // 双眼始终叠在贴图之上，避免图集路径丢失身份。
        {
            let eye_y = oy + bh * 0.28;
            let eye_dx = if self.facing >= 0.0 { 1.5 } else { -1.5 };
            draw.fill_rect(
                Rect::new(ox + bw * 0.22 + eye_dx, eye_y, 3.5, 3.5),
                Color::rgb(0.95, 0.92, 1.0),
            );
            draw.fill_rect(
                Rect::new(ox + bw * 0.62 + eye_dx, eye_y, 3.5, 3.5),
                Color::rgb(0.95, 0.92, 1.0),
            );
            draw.fill_rect(
                Rect::new(ox + bw * 0.28 + eye_dx, eye_y + 1.0, 1.8, 1.8),
                Color::rgb(0.12, 0.08, 0.18),
            );
            draw.fill_rect(
                Rect::new(ox + bw * 0.68 + eye_dx, eye_y + 1.0, 1.8, 1.8),
                Color::rgb(0.12, 0.08, 0.18),
            );
        }

        // 头顶身份短标（受伤闪时更亮）
        let tag_a = if flash { 0.85 } else { 0.55 + 0.25 * telegraph };
        draw.fill_rect(
            Rect::new(ox + bw * 0.35, oy - 4.0, bw * 0.3, 2.0),
            Color::rgba(rim.r, rim.g, rim.b, tag_a),
        );

        let ratio = (self.hp / self.max_hp).clamp(0.0, 1.0);
        draw.fill_rect(
            Rect::new(sx, sy - 6.0, SLIME_W, 3.0),
            Color::rgb(0.12, 0.08, 0.14),
        );
        draw.fill_rect(
            Rect::new(sx, sy - 6.0, SLIME_W * ratio, 3.0),
            Color::rgb(0.85, 0.28, 0.42),
        );
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
        if e.kind.flies() {
            let ecy = e.y + eh * 0.5;
            let pcy = py + ph * 0.5;
            let dy = pcy - ecy;
            let tx = wrap_tx((ecx / TILE).floor() as i32);
            let hover = world.surface_at(tx) as f32 * TILE - TILE * 5.5;
            let chase = dx.abs() < TILE * 22.0;
            let speed = if chase {
                TILE * 4.6 * aggro
            } else {
                TILE * 2.2
            };
            e.vx = speed * e.facing;
            e.vy = if chase {
                dy.signum() * TILE * 2.6 + (e.bob * 2.1).sin() * TILE * 0.3
            } else {
                (hover - e.y).clamp(-TILE * 3.0, TILE * 3.0) * 1.4
                    + e.bob.sin() * TILE * 0.35
            };
        } else {
            let chase_r = TILE * (12.0 + night * 8.0);
            let base_speed = match e.kind {
                EnemyKind::Zombie => TILE * 2.8,
                EnemyKind::ShadowSlime | EnemyKind::DemonEye => TILE * 3.5,
            };
            let wander = match e.kind {
                EnemyKind::Zombie => TILE * 1.6,
                EnemyKind::ShadowSlime | EnemyKind::DemonEye => TILE * 1.2,
            };
            let mut speed = if dx.abs() < chase_r {
                base_speed * aggro * e.facing
            } else {
                wander * e.facing
            };
            if e.windup_t > 0.0 {
                speed *= 0.15;
            }
            e.vx = speed;
            e.vy += GRAVITY * dt;
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
