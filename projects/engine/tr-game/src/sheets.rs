//! `Tiles_N.xnb` / `Item_N.xnb` 文件名里的编号。
//!
//! 这些数字来自文件名，不是夹具 `BlockId` / `ItemId`。

use tr_core::{BlockId, ItemId, WallId};

/// 夹具方块对应的 `Tiles_{n}.xnb`。没有对应文件时不画这张图。
pub(crate) fn tile_file(id: BlockId) -> Option<u32> {
    Some(match id {
        BlockId::DIRT => 0,
        BlockId::STONE => 1,
        BlockId::GRASS => 2,
        BlockId::TORCH => 4,
        BlockId::IRON_ORE => 6,
        BlockId::COPPER_ORE => 7,
        BlockId::FURNACE => 17,
        BlockId::WORKBENCH => 18,
        BlockId::PLATFORM => 19,
        BlockId::SAPLING => 20,
        BlockId::CHEST => 21,
        BlockId::WOOD => 30,
        BlockId::SAND => 53,
        BlockId::BED => 79,
        BlockId::SNOW => 147,
        BlockId::LEAF => 192,
        BlockId::LADDER | BlockId::ROPE => 213,
        _ => return None,
    })
}

/// 夹具物品对应的 `Item_{n}.xnb`。
pub(crate) fn item_file(id: ItemId) -> Option<u32> {
    Some(match id {
        ItemId::DIRT => 2,
        ItemId::STONE => 3,
        ItemId::TORCH => 8,
        ItemId::WOOD => 9,
        ItemId::IRON_ORE => 11,
        ItemId::COPPER_ORE => 12,
        ItemId::COPPER_BAR => 20,
        ItemId::IRON_BAR => 22,
        ItemId::GEL => 23,
        ItemId::WOOD_SWORD => 24,
        ItemId::SAPLING => 27,
        ItemId::FURNACE => 33,
        ItemId::WORKBENCH => 36,
        ItemId::WOOD_BOW => 39,
        ItemId::WOOD_ARROW => 40,
        ItemId::CHEST => 48,
        ItemId::CLOUD_BOTTLE => 53,
        ItemId::GRAPPLE => 84,
        ItemId::PLATFORM => 94,
        ItemId::WOOD_WALL => 93,
        ItemId::STONE_WALL => 26,
        ItemId::COPPER_COIN => 71,
        _ => return None,
    })
}

/// 夹具墙对应的 `Wall_{n}.xnb`（文件名编号，非夹具 `WallId`）。
pub(crate) fn wall_file(id: WallId) -> Option<u32> {
    Some(match id {
        WallId::STONE => 1,
        WallId::DIRT => 2,
        WallId::WOOD => 4,
        _ => return None,
    })
}

/// 史莱姆外形。这张是灰度，绘制时再乘身体色。
pub(crate) const SLIME_NPC_FILE: u32 = 1;
/// 正版恶魔眼 `NPC_2`。
pub(crate) const DEMON_EYE_NPC_FILE: u32 = 2;
/// 正版僵尸 `NPC_3`。
pub(crate) const ZOMBIE_NPC_FILE: u32 = 3;
