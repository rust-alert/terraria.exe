//! 贴图文件编号查询。方块 / 物品 / 墙 / NPC 均来自内容注册表。

use tr_core::{ItemId, NpcId, WallId};

/// 物品对应的 `Item_{n}.xnb`。
pub(crate) fn item_file(id: ItemId) -> Option<u32> {
    id.texture_file()
}

/// 墙对应的 `Wall_{n}.xnb`。
pub(crate) fn wall_file(id: WallId) -> Option<u32> {
    id.texture_file()
}

/// NPC 对应的 `NPC_{n}.xnb`。
pub(crate) fn npc_file(id: NpcId) -> Option<u32> {
    id.texture_file()
}
