//! 武器参数类型。具体数值由内容模块登记到物品上。

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
    /// 若该物品是武器则返回战斗参数。只读内容表。
    pub fn weapon(self) -> Option<WeaponStats> {
        crate::try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.weapon)
    }

    pub fn is_weapon(self) -> bool {
        self.weapon().is_some()
    }

    /// 显示名：只读内容表。
    pub fn label(self) -> String {
        if let Some(c) = crate::try_content() {
            if let Some(d) = c.item(self) {
                return d.name.clone();
            }
        }
        "未知".to_string()
    }
}
