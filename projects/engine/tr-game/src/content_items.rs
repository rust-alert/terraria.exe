//! 当前会用物品的图标文件号登记。
//!
//! 物品数值身份仍是过渡期本地编号。`item_file` 只存正版 `Item_N` 文件号，
//! 不在各渲染模块里手写对照。完整 Item ID 对齐另开切片。

use tr_core::{BlockId, ContentModule, ContentRegistry, ItemId, WallId};

/// 随客户端发布的物品图标登记。
pub struct BootstrapItems;

impl ContentModule for BootstrapItems {
    fn name(&self) -> &str {
        "bootstrap_items"
    }

    fn register(&self, registry: &mut ContentRegistry) -> Result<(), String> {
        let rows: &[(u32, &str, &str, u32)] = &[
            (ItemId::DIRT.0, "terraria:dirt", "泥土", 2),
            (ItemId::STONE.0, "terraria:stone", "石头", 3),
            (ItemId::TORCH.0, "terraria:torch", "火把", 8),
            (ItemId::WOOD.0, "terraria:wood", "木材", 9),
            (ItemId::IRON_ORE.0, "terraria:iron_ore", "铁矿", 11),
            (ItemId::COPPER_ORE.0, "terraria:copper_ore", "铜矿", 12),
            (ItemId::COPPER_BAR.0, "terraria:copper_bar", "铜锭", 20),
            (ItemId::IRON_BAR.0, "terraria:iron_bar", "铁锭", 22),
            (ItemId::GEL.0, "terraria:gel", "凝胶", 23),
            (ItemId::WOOD_SWORD.0, "terraria:wood_sword", "木剑", 24),
            (ItemId::SAPLING.0, "terraria:sapling", "树苗", 27),
            (ItemId::STONE_WALL.0, "terraria:stone_wall", "石墙", 26),
            (ItemId::FURNACE.0, "terraria:furnace", "熔炉", 33),
            (ItemId::WORKBENCH.0, "terraria:workbench", "工作台", 36),
            (ItemId::WOOD_BOW.0, "terraria:wood_bow", "木弓", 39),
            (ItemId::WOOD_ARROW.0, "terraria:wood_arrow", "木箭", 40),
            (ItemId::CHEST.0, "terraria:chest", "木箱", 48),
            (ItemId::CLOUD_BOTTLE.0, "terraria:cloud_bottle", "凝胶云瓶", 53),
            (ItemId::COPPER_COIN.0, "terraria:copper_coin", "铜币", 71),
            (ItemId::GRAPPLE.0, "terraria:grapple", "钩爪", 84),
            (ItemId::WOOD_WALL.0, "terraria:wood_wall", "木墙", 93),
            (ItemId::PLATFORM.0, "terraria:platform", "木平台", 94),
        ];
        for &(id, key, name, file) in rows {
            let mut b = registry.item_entry(id).key(key).name(name).item_file(file);
            b = match id {
                x if x == ItemId::DIRT.0 => b.places(BlockId::DIRT),
                x if x == ItemId::STONE.0 => b.places(BlockId::STONE),
                x if x == ItemId::WOOD.0 => b.places(BlockId::WOOD),
                x if x == ItemId::TORCH.0 => b.places(BlockId::TORCH),
                x if x == ItemId::SAPLING.0 => b.places(BlockId::SAPLING),
                x if x == ItemId::PLATFORM.0 => b.places(BlockId::PLATFORM),
                x if x == ItemId::CHEST.0 => b.places(BlockId::CHEST),
                x if x == ItemId::FURNACE.0 => b.places(BlockId::FURNACE),
                x if x == ItemId::WORKBENCH.0 => b.places(BlockId::WORKBENCH),
                x if x == ItemId::WOOD_WALL.0 => b.wall(WallId::WOOD),
                x if x == ItemId::STONE_WALL.0 => b.wall(WallId::STONE),
                _ => b,
            };
            b.register()?;
        }
        Ok(())
    }
}

/// 正版墙类型登记。
pub struct VanillaWalls;

impl ContentModule for VanillaWalls {
    fn name(&self) -> &str {
        "vanilla_walls"
    }

    fn register(&self, registry: &mut ContentRegistry) -> Result<(), String> {
        registry
            .wall_entry(1)
            .key("terraria:stone_wall")
            .name("石墙")
            .max_hp(70)
            .drop(ItemId::STONE)
            .register()?;
        registry
            .wall_entry(2)
            .key("terraria:dirt_wall")
            .name("泥土墙")
            .max_hp(40)
            .drop(ItemId::DIRT)
            .register()?;
        registry
            .wall_entry(4)
            .key("terraria:wood_wall")
            .name("木墙")
            .max_hp(50)
            .drop(ItemId::WOOD)
            .register()?;
        Ok(())
    }
}
