//! 贴图文件编号查询。方块 / 物品 / 墙均来自内容注册表，这里不再按类型枚举。

use tr_core::{ItemId, WallId};

/// 物品对应的 `Item_{n}.xnb`。
pub(crate) fn item_file(id: ItemId) -> Option<u32> {
    id.texture_file()
}

/// 墙对应的 `Wall_{n}.xnb`。
pub(crate) fn wall_file(id: WallId) -> Option<u32> {
    id.texture_file()
}

/// 史莱姆外形。这张是灰度，绘制时再乘身体色。
pub(crate) const SLIME_NPC_FILE: u32 = 1;
/// 恶魔眼 `NPC_2`。
pub(crate) const DEMON_EYE_NPC_FILE: u32 = 2;
/// 僵尸 `NPC_3`。
pub(crate) const ZOMBIE_NPC_FILE: u32 = 3;
