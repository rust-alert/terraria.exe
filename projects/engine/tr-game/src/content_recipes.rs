//! 过渡期配方模块。通过 [`ContentModule`] 登记，删除平行 `RECIPES` 数组。
//!
//! 材料与产物仍使用当前物品编号空间。原版 Item ID 对齐后只改登记，不改制作结算。

use tr_core::{
    ContentModule, ContentRegistry, ItemId, RecipeStation,
};

/// 启动时登记当前可玩配方。
pub struct BootstrapRecipes;

impl ContentModule for BootstrapRecipes {
    fn name(&self) -> &str {
        "bootstrap_recipes"
    }

    fn register(&self, registry: &mut ContentRegistry) -> Result<(), String> {
        registry
            .recipe(ItemId::WORKBENCH, 1)
            .key("terraria:workbench")
            .label("工作台 ×1")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 10)
            .register()?;
        registry
            .recipe(ItemId::WOOD_PICK, 1)
            .key("terraria:wood_pick")
            .label("木镐 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 10)
            .register()?;
        registry
            .recipe(ItemId::STONE_PICK, 1)
            .key("terraria:stone_pick")
            .label("石镐 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 4)
            .ingredient(ItemId::STONE, 10)
            .register()?;
        registry
            .recipe(ItemId::WOOD_HAMMER, 1)
            .key("terraria:wood_hammer")
            .label("木锤 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 6)
            .register()?;
        registry
            .recipe(ItemId::WOOD_SWORD, 1)
            .key("terraria:wood_sword")
            .label("木剑 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 6)
            .register()?;
        registry
            .recipe(ItemId::PLATFORM, 2)
            .key("terraria:platform")
            .label("木平台 ×2")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 2)
            .register()?;
        registry
            .recipe(ItemId::WOOD_WALL, 4)
            .key("terraria:wood_wall")
            .label("木墙 ×4")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 1)
            .register()?;
        registry
            .recipe(ItemId::STONE_WALL, 4)
            .key("terraria:stone_wall")
            .label("石墙 ×4")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::STONE, 1)
            .register()?;
        registry
            .recipe(ItemId::CHEST, 1)
            .key("terraria:chest")
            .label("木箱 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 8)
            .register()?;
        registry
            .recipe(ItemId::TORCH, 3)
            .key("terraria:torch")
            .label("火把 ×3")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 1)
            .ingredient(ItemId::GEL, 1)
            .register()?;
        registry
            .recipe(ItemId::LADDER, 4)
            .key("terraria:ladder")
            .label("木梯 ×4")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 2)
            .register()?;
        registry
            .recipe(ItemId::ROPE, 8)
            .key("terraria:rope")
            .label("绳索 ×8")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 1)
            .register()?;
        registry
            .recipe(ItemId::WOOD_ARMOR, 1)
            .key("terraria:wood_armor")
            .label("木甲 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 12)
            .ingredient(ItemId::GEL, 4)
            .register()?;
        registry
            .recipe(ItemId::WOOD_BOW, 1)
            .key("terraria:wood_bow")
            .label("木弓 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 8)
            .ingredient(ItemId::STONE, 2)
            .register()?;
        registry
            .recipe(ItemId::WOOD_ARROW, 5)
            .key("terraria:wood_arrow")
            .label("木箭 ×5")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 1)
            .register()?;
        registry
            .recipe(ItemId::GEL_STAFF, 1)
            .key("terraria:gel_staff")
            .label("凝胶法杖 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 4)
            .ingredient(ItemId::GEL, 6)
            .ingredient(ItemId::STONE, 2)
            .register()?;
        registry
            .recipe(ItemId::FURNACE, 1)
            .key("terraria:furnace")
            .label("熔炉 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::STONE, 16)
            .register()?;
        registry
            .recipe(ItemId::BED, 1)
            .key("terraria:bed")
            .label("床 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 10)
            .ingredient(ItemId::GEL, 2)
            .register()?;
        registry
            .recipe(ItemId::COPPER_BAR, 1)
            .key("terraria:smelt_copper")
            .label("铜锭 ×1")
            .station(RecipeStation::Furnace)
            .ingredient(ItemId::COPPER_ORE, 3)
            .register()?;
        registry
            .recipe(ItemId::IRON_BAR, 1)
            .key("terraria:smelt_iron")
            .label("铁锭 ×1")
            .station(RecipeStation::Furnace)
            .ingredient(ItemId::IRON_ORE, 3)
            .register()?;
        registry
            .recipe(ItemId::COPPER_PICK, 1)
            .key("terraria:copper_pick")
            .label("铜镐 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::COPPER_BAR, 6)
            .ingredient(ItemId::WOOD, 4)
            .register()?;
        registry
            .recipe(ItemId::CLOTH_BAG, 1)
            .key("terraria:cloth_bag")
            .label("布袋 ×1")
            .station(RecipeStation::Hand)
            .ingredient(ItemId::WOOD, 4)
            .ingredient(ItemId::GEL, 2)
            .register()?;
        registry
            .recipe(ItemId::TRAVEL_PACK, 1)
            .key("terraria:travel_pack")
            .label("旅行包 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 10)
            .ingredient(ItemId::GEL, 4)
            .ingredient(ItemId::STONE, 2)
            .register()?;
        registry
            .recipe(ItemId::GRAPPLE, 1)
            .key("terraria:grapple")
            .label("钩爪 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::WOOD, 6)
            .ingredient(ItemId::STONE, 4)
            .register()?;
        registry
            .recipe(ItemId::CLOUD_BOTTLE, 1)
            .key("terraria:cloud_bottle")
            .label("凝胶云瓶 ×1")
            .station(RecipeStation::Workbench)
            .ingredient(ItemId::GEL, 6)
            .ingredient(ItemId::STONE, 2)
            .register()?;
        Ok(())
    }
}
