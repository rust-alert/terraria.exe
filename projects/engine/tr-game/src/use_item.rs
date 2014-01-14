//! 统一「使用当前手持物」：左键主用，右键次要用。

use tr_core::{ItemId, WeaponKind};

use crate::enemy::{Enemy, try_melee};
use crate::fx::{DamageFloater, DustParticle};
use crate::player::Player;
use crate::weapon;
use crate::world::World;

/// 主用结果，供宿主播特效与 toast。
#[derive(Debug)]
pub enum PrimaryOutcome {
    Dig { msg: String, tx: i32, ty: i32 },
    Place { msg: String, tx: i32, ty: i32 },
    Melee(String),
    Ranged(String),
    Eat(String),
}

/// 左键：使用当前手持物。
pub fn primary_use(
    player: &mut Player,
    world: &mut World,
    enemies: &mut Vec<Enemy>,
    damage_fx: &mut Vec<DamageFloater>,
    dust_fx: &mut Vec<DustParticle>,
    projectiles: &mut Vec<weapon::Projectile>,
    tx: i32,
    ty: i32,
    aim_x: f32,
    aim_y: f32,
    pressed: bool,
    held: bool,
) -> Option<PrimaryOutcome> {
    let sel = player.selected_item();

    // 钩爪由 Q 专用，左键忽略。
    if sel.is_grapple() {
        return None;
    }

    // 食物：按下沿食用。
    if pressed && sel.heal_amount().is_some() {
        return player.try_eat().map(PrimaryOutcome::Eat);
    }

    // 可放置方块 / 树苗：按下沿放置。
    if pressed && (sel.as_block().is_some() || sel == ItemId::SAPLING) {
        return player
            .try_place(world, tx, ty)
            .map(|msg| PrimaryOutcome::Place { msg, tx, ty });
    }

    // 锤：只拆墙。
    if sel.is_hammer() {
        if !held || !player.can_swing() {
            return None;
        }
        return player
            .dig_wall_at(world, tx, ty)
            .map(|msg| PrimaryOutcome::Dig { msg, tx, ty });
    }

    let wpn = sel.weapon();
    let is_ranged = wpn
        .map(|w| matches!(w.kind, WeaponKind::Ranged | WeaponKind::Magic))
        .unwrap_or(false);

    if is_ranged {
        if !held || !player.can_swing() {
            return None;
        }
        return weapon::try_fire(player, world, aim_x, aim_y, projectiles)
            .map(PrimaryOutcome::Ranged);
    }

    // 近战优先打怪，否则挖掘。
    if sel.melee_damage().is_some() || sel.is_tool() {
        if !held || !player.can_swing() {
            return None;
        }
        if let Some(msg) = try_melee(player, enemies, world, damage_fx, dust_fx) {
            player.mark_swing();
            let tool = player.selected_item();
            let mut full = msg;
            if let Some(broke) = player.inv.wear_tool(tool, 1) {
                full = format!("{full}；{broke}");
            }
            return Some(PrimaryOutcome::Melee(full));
        }
        if sel.mine_power().is_some() || !sel.is_tool() {
            return player
                .dig_hold(world, tx, ty)
                .map(|msg| PrimaryOutcome::Dig { msg, tx, ty });
        }
        return None;
    }

    if held {
        return player
            .dig_hold(world, tx, ty)
            .map(|msg| PrimaryOutcome::Dig { msg, tx, ty });
    }
    None
}

/// 右键次要使用：手持墙材时铺背景墙。
pub fn secondary_use(player: &mut Player, world: &mut World, tx: i32, ty: i32) -> Option<String> {
    let sel = player.selected_item();
    if sel.as_wall().is_some() {
        return player.try_place_wall(world, tx, ty);
    }
    // 锤持有时右键也可铺墙（需背包有墙材——仍用手持材料语义，故仅墙材生效）。
    None
}
