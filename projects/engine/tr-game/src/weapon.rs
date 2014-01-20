//! 武器使用：近战已由 `enemy::try_melee`；此处管投射物。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{DamageHit, DamageType, ItemId, WeaponKind, WeaponStats};

use crate::enemy::Enemy;
use crate::player::Player;
use crate::world::{TILE, WORLD_H, World, world_pixel_w, wrap_delta_x, wrap_tx, wrap_xf};

#[derive(Debug, Clone)]
pub struct Projectile {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub damage: f32,
    pub dtype: DamageType,
    pub knockback: f32,
    pub life: f32,
    pub color: Color,
}

impl Projectile {
    pub fn draw(&self, draw: &mut DrawList, cam_x: f32, cam_y: f32) {
        let mut sx = self.x - cam_x;
        let w = world_pixel_w();
        if sx > w * 0.5 {
            sx -= w;
        } else if sx < -w * 0.5 {
            sx += w;
        }
        let sy = self.y - cam_y;
        let size = if matches!(self.dtype, DamageType::Elemental) {
            7.0
        } else {
            5.0
        };
        draw.fill_rect(
            Rect::new(sx - size * 0.5, sy - size * 0.5, size, size),
            self.color,
        );
    }
}

/// 朝世界坐标瞄准点开火；成功时推入 `out` 并扣弹药 / 魔力。
pub fn try_fire(
    player: &mut Player,
    world: &World,
    aim_x: f32,
    aim_y: f32,
    out: &mut Vec<Projectile>,
) -> Option<String> {
    let item = player.selected_item();
    let Some(wpn) = item.weapon() else {
        return None;
    };
    if !matches!(wpn.kind, WeaponKind::Ranged | WeaponKind::Magic) {
        return None;
    }
    if !player.can_swing() {
        return None;
    }
    if player.inv.get(item) == 0 {
        return Some(format!("没有{}", item.label()));
    }
    if let Some(ammo) = wpn.ammo {
        if player.inv.get(ammo) == 0 {
            return Some(format!("缺少{}", ammo.label()));
        }
    }
    if wpn.mana_cost > 0.0 && !player.try_spend_mana(wpn.mana_cost) {
        return Some(format!("魔力不足（需 {:.0}）", wpn.mana_cost));
    }

    let (px, py, pw, ph) = player.hitbox();
    let ox = px + pw * 0.5;
    let oy = py + ph * 0.35;
    let dx = wrap_delta_x(ox, aim_x);
    let dy = aim_y - oy;
    let len = dx.hypot(dy).max(1.0);
    let speed = wpn.reach_or_speed * TILE;
    let vx = dx / len * speed;
    let vy = dy / len * speed;
    // 面朝方向同步
    if dx.abs() > 1.0 {
        player.facing = dx.signum();
    }

    if let Some(ammo) = wpn.ammo {
        let _ = player.inv.try_take(ammo, 1);
    }

    let color = match wpn.dtype {
        DamageType::Elemental => Color::rgb(0.55, 0.95, 1.0),
        DamageType::Kinetic => Color::rgb(0.9, 0.78, 0.45),
        DamageType::Thermal => Color::rgb(1.0, 0.45, 0.2),
        DamageType::Energy => Color::rgb(0.6, 0.75, 1.0),
        DamageType::Toxin => Color::rgb(0.45, 0.9, 0.35),
        DamageType::Gravity => Color::rgb(0.7, 0.4, 0.95),
        DamageType::True => Color::rgb(1.0, 1.0, 1.0),
    };

    out.push(Projectile {
        x: ox,
        y: oy,
        vx,
        vy,
        damage: wpn.damage,
        dtype: wpn.dtype,
        knockback: wpn.knockback,
        life: 1.8,
        color,
    });
    player.mark_swing_for(wpn.interval);
    let _ = world; // 预留：日后穿墙判定按武器
    let kind = wpn.kind.name();
    let dtype = wpn.dtype.name();
    Some(format!("{kind}·{dtype} −{:.0}", wpn.damage))
}

pub fn update_projectiles(
    world: &mut World,
    projectiles: &mut Vec<Projectile>,
    enemies: &mut Vec<Enemy>,
    floaters: &mut Vec<crate::fx::DamageFloater>,
    dust: &mut Vec<crate::fx::DustParticle>,
    dt: f32,
) -> Option<String> {
    let mut kill_msg = None;
    projectiles.retain_mut(|p| {
        p.life -= dt;
        if p.life <= 0.0 {
            return false;
        }
        p.x = wrap_xf(p.x + p.vx * dt);
        p.y += p.vy * dt;
        if p.y < 0.0 || p.y > WORLD_H as f32 * TILE {
            return false;
        }
        let tx = wrap_tx((p.x / TILE).floor() as i32);
        let ty = (p.y / TILE).floor() as i32;
        if world.in_bounds(tx, ty) && world.get(tx, ty).blocks_motion() {
            return false;
        }

        let hit = DamageHit::new(p.damage, p.dtype);
        for e in enemies.iter_mut() {
            if e.hurt_cd > 0.0 {
                continue;
            }
            let (ex, ey, ew, eh) = e.hitbox();
            if p.x >= ex && p.x <= ex + ew && p.y >= ey && p.y <= ey + eh {
                let actual = e.apply_hit(hit, p.knockback * (if p.vx >= 0.0 { 1.0 } else { -1.0 }));
                crate::fx::push_hit(floaters, ex + ew * 0.5, ey, actual);
                if e.hp <= 0.0 {
                    let ecx = ex + ew * 0.5;
                    let ecy = ey + eh * 0.5;
                    let etx = wrap_tx((ecx / TILE).floor() as i32);
                    let ety = (ecy / TILE).floor() as i32;
                    e.drop_loot(world, dust, ecx, ecy, etx, ety);
                    kill_msg = Some(format!("击杀（{} −{actual:.0}）", p.dtype.name()));
                } else {
                    kill_msg = Some(format!("{} −{actual:.0}", p.dtype.name()));
                }
                return false;
            }
        }
        true
    });
    enemies.retain(|e| e.hp > 0.0);
    kill_msg
}

pub fn draw_projectiles(projectiles: &[Projectile], draw: &mut DrawList, cam_x: f32, cam_y: f32) {
    for p in projectiles {
        p.draw(draw, cam_x, cam_y);
    }
}

/// 近战挥击参数（从手持武器读取）。
pub fn active_melee(player: &Player) -> Option<WeaponStats> {
    let w = player.selected_item().weapon()?;
    (w.kind == WeaponKind::Melee).then_some(w)
}
