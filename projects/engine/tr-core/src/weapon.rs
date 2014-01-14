//! 武器表：近战 / 远程 / 魔力。

use crate::ItemId;
use crate::damage::DamageType;

/// 武器使用方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponKind {
    Melee,
    Ranged,
    Magic,
}

impl WeaponKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Melee => "近战",
            Self::Ranged => "远程",
            Self::Magic => "魔力",
        }
    }
}

/// 一把武器的战斗参数。
#[derive(Debug, Clone, Copy)]
pub struct WeaponStats {
    pub kind: WeaponKind,
    pub damage: f32,
    pub dtype: DamageType,
    /// 挥击 / 射击间隔（秒）。
    pub interval: f32,
    /// 近战触及（格）；远程 / 魔力为弹速（格/秒）。
    pub reach_or_speed: f32,
    /// 击退（相对 `TILE` 的倍数，由调用方换算）。
    pub knockback: f32,
    /// 魔力消耗；非魔力武器为 0。
    pub mana_cost: f32,
    /// 远程弹药；无则为每次不耗弹。
    pub ammo: Option<ItemId>,
}

impl ItemId {
    /// 若该物品是武器则返回战斗参数（优先内容表）。
    pub fn weapon(self) -> Option<WeaponStats> {
        if let Some(c) = crate::try_content() {
            if let Some(d) = c.item(self) {
                if d.weapon.is_some() {
                    return d.weapon;
                }
            }
        }
        match self {
            Self::WOOD_SWORD => Some(WeaponStats {
                kind: WeaponKind::Melee,
                damage: 18.0,
                dtype: DamageType::Kinetic,
                interval: 0.22,
                reach_or_speed: 2.2,
                knockback: 10.0,
                mana_cost: 0.0,
                ammo: None,
            }),
            Self::WOOD_PICK | Self::STONE_PICK | Self::COPPER_PICK => Some(WeaponStats {
                kind: WeaponKind::Melee,
                damage: 8.0,
                dtype: DamageType::Kinetic,
                interval: 0.22,
                reach_or_speed: 2.0,
                knockback: 6.0,
                mana_cost: 0.0,
                ammo: None,
            }),
            Self::WOOD_BOW => Some(WeaponStats {
                kind: WeaponKind::Ranged,
                damage: 14.0,
                dtype: DamageType::Kinetic,
                interval: 0.38,
                reach_or_speed: 22.0,
                knockback: 4.0,
                mana_cost: 0.0,
                ammo: Some(Self::WOOD_ARROW),
            }),
            Self::GEL_STAFF => Some(WeaponStats {
                kind: WeaponKind::Magic,
                damage: 22.0,
                dtype: DamageType::Elemental,
                interval: 0.42,
                reach_or_speed: 16.0,
                knockback: 3.0,
                mana_cost: 12.0,
                ammo: None,
            }),
            _ => None,
        }
    }

    pub fn is_weapon(self) -> bool {
        self.weapon().is_some()
    }

    /// 显示名：内容表优先，否则常量回退。
    pub fn label(self) -> String {
        if let Some(c) = crate::try_content() {
            if let Some(d) = c.item(self) {
                return d.name.clone();
            }
        }
        self.name().to_string()
    }
}
