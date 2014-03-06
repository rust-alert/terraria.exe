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

/// 公开的 `Main.npcFrameCount[type]`（竖条帧数）。未知类型按 1 帧整图。
pub(crate) fn npc_frame_count(id: NpcId) -> u32 {
    match id {
        NpcId::BLUE_SLIME => 2,
        NpcId::DEMON_EYE => 2,
        NpcId::ZOMBIE => 3,
        // `NPC_22` 高 1456 = 26×56，`NPC_17` 高 1400 = 25×56。
        NpcId::GUIDE => 26,
        NpcId::MERCHANT => 25,
        _ => 1,
    }
}
