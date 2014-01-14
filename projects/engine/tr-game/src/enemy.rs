//! 地表敌怪：暗影凝胶（竖切最小战斗环）。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{DamageHit, ItemId, ResistProfile, resolve_damage};

use crate::player::Player;
use crate::world::{
    TILE, WORLD_H, WORLD_W, World, tile_x_near, world_pixel_w, wrap_delta_x, wrap_tx, wrap_xf,
};

const GRAVITY: f32 = TILE * 40.0;
const SLIME_W: f32 = TILE * 0.9;
const SLIME_H: f32 = TILE * 0.7;
const TOUCH_DAMAGE: f32 = 8.0;
const TOUCH_CD: f32 = 0.85;
const WINDUP_TIME: f32 = 0.32;

/// 敌怪身份（视觉与抗性绑定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// 暗影凝胶：地表夜行软体。
    ShadowSlime,
}

impl EnemyKind {
    fn label(self) -> &'static str {
        match self {
            Self::ShadowSlime => "暗影凝胶",
        }
    }

    fn body(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.42, 0.16, 0.58),
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.72, 0.38, 0.95),
        }
    }

    fn core(self) -> Color {
        match self {
            Self::ShadowSlime => Color::rgb(0.95, 0.78, 1.0),
        }
    }

    fn resist(self) -> ResistProfile {
        match self {
            Self::ShadowSlime => ResistProfile::slime(),
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
        (self.x, self.y, SLIME_W, SLIME_H)
    }

    pub fn draw(
        &self,
        draw: &mut DrawList,
        cam_x: f32,
        cam_y: f32,
        slime: Option<crate::tiles::TileView>,
    ) {
        let sx = self.x - cam_x;
        // 跨缝：把怪画到最近相机像
        let sx = {
            let w = world_pixel_w();
            let mut d = sx;
            if d > w * 0.5 {
                d -= w;
            } else if d < -w * 0.5 {
                d += w;
            }
            d
        };
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
                Color::rgb(1.0, 1.0, 1.0)
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

/// 在地表远离逃生舱处刷几只凝胶。
pub fn spawn_surface_slimes(world: &World, out: &mut Vec<Enemy>) {
    out.clear();
    let mut x = 40;
    while x < WORLD_W - 4 {
        let tx = wrap_tx(x);
        let near_pod = {
            let t = 26;
            let dist = (tx - t)
                .rem_euclid(WORLD_W)
                .min((t - tx).rem_euclid(WORLD_W));
            dist < 18
        };
        if !near_pod && (tx as u64 * 17 + world.seed) % 11 == 0 {
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
        let sh = world.surface_at(55);
        out.push(Enemy::slime(55.0 * TILE, sh as f32 * TILE - SLIME_H));
        let sh2 = world.surface_at(90);
        out.push(Enemy::slime(90.0 * TILE, sh2 as f32 * TILE - SLIME_H));
    }
}

pub fn update_enemies(
    world: &World,
    enemies: &mut Vec<Enemy>,
    player: &mut Player,
    dt: f32,
    night: f32,
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

        let ecx = e.x + SLIME_W * 0.5;
        let dx = wrap_delta_x(ecx, pcx);
        e.facing = if dx.abs() > 4.0 {
            dx.signum()
        } else {
            e.facing
        };
        let chase_r = TILE * (12.0 + night * 8.0);
        let mut speed = if dx.abs() < chase_r {
            TILE * 3.5 * aggro * e.facing
        } else {
            TILE * 1.2 * e.facing
        };
        if e.windup_t > 0.0 {
            speed *= 0.15;
        }
        e.vx = speed;
        e.vy += GRAVITY * dt;

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
                    player.hurt_hit(DamageHit::kinetic(TOUCH_DAMAGE * dmg_mul));
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
    e.x += dx;
    e.y += dy;
    if dx != 0.0 {
        e.x = wrap_xf(e.x);
    }
    let min_tx = (e.x / TILE).floor() as i32 - 1;
    let max_tx = ((e.x + SLIME_W) / TILE).floor() as i32 + 1;
    let min_ty = (e.y / TILE).floor() as i32;
    let max_ty = ((e.y + SLIME_H) / TILE).floor() as i32;
    for ty in min_ty..=max_ty {
        if ty < 0 || ty >= WORLD_H {
            continue;
        }
        for tx in min_tx..=max_tx {
            if !world.get(tx, ty).blocks_motion() {
                continue;
            }
            let bx = tile_x_near(tx, e.x + SLIME_W * 0.5);
            let by = ty as f32 * TILE;
            if !aabb(e.x, e.y, SLIME_W, SLIME_H, bx, by, TILE, TILE) {
                continue;
            }
            if dx > 0.0 {
                e.x = wrap_xf(bx - SLIME_W);
                e.vx = 0.0;
                e.facing = -1.0;
            } else if dx < 0.0 {
                e.x = wrap_xf(bx + TILE);
                e.vx = 0.0;
                e.facing = 1.0;
            }
            if dy > 0.0 {
                e.y = by - SLIME_H;
                e.vy = 0.0;
            } else if dy < 0.0 {
                e.y = by + TILE;
                e.vy = 0.0;
            }
        }
    }
    e.y = e.y.clamp(0.0, WORLD_H as f32 * TILE - SLIME_H);
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
            world.spawn_drop_at_tile(tx, ty, ItemId::GEL, 2);
            world.spawn_drop_at_tile(tx, ty, ItemId::SCRAP, 1);
            crate::fx::burst_gel(dust, ecx, ecy);
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
