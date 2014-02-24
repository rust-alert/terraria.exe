#![warn(missing_docs)]
//! Terraria 游戏层基类型、方块与物品表。

mod biome;
mod content;
mod content_module;
mod damage;
mod fluid;
mod recipe;
mod schema;
mod tile;
mod tile_sets;
mod weapon;
mod wld;

pub use biome::{BiomeId, biome_at};
pub use content::{
    BlockDef, BlockFaceKind, ContentRegistry, ItemDef, WallDef, block_def, content, install,
    install_builtin_fixture, is_installed, item_def, try_content,
};
pub use content_module::{
    ContentModule, ItemRegistration, TileRegistration, WallRegistration, boot_content_modules,
    tile_sets_from_registry,
};
pub use damage::{DamageHit, DamageType, ResistProfile, resolve_damage};
pub use fluid::FluidLevel;
pub use recipe::{RecipeDef, RecipeRegistration, RecipeStation};
pub use schema::{
    SchemaToolKind, block_def_from_schema, consumable_item_from_schema, example_dirt_block,
    placeable_item_from_schema, tool_item_from_schema,
};
pub use tile::{LiquidKind, SlopeKind, Tile};
pub use tile_sets::{TileSets, install_tile_sets, try_tile_sets};
pub use weapon::{WeaponKind, WeaponStats};
pub use wld::{
    WORLD_VERSION_1_4_5, WORLD_VERSION_1_4_5_8, WldCell, WldError, WldFileHeader, WldProperties,
    WldSeedFlags, read_file_header, read_tiles, read_world_properties,
};

use std::fmt;
use std::sync::Arc;

use spark_core::{ErrorArg, ErrorArgs};

/// 方块标识。数值是类型 id。
///
/// 空气在原版里是格子未激活，不是类型 0。类型 0 是泥土。
/// 下面的常量只覆盖当前玩法还在按名字引用的类型。属性查 [`crate::TileSets`]，
/// 玩法行由 [`crate::ContentModule`] 登记。禁止再维护仓库内表文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl BlockId {
    /// 格子未激活。不是物块类型。正版类型 0 是泥土。
    pub const AIR: Self = Self(u32::MAX);
    /// 泥土。正版 `TileID.Dirt` = 0。
    pub const DIRT: Self = Self(0);
    /// 石头。正版 `TileID.Stone` = 1。
    pub const STONE: Self = Self(1);
    /// 草。正版 `TileID.Grass` = 2。
    pub const GRASS: Self = Self(2);
    /// 火把。正版 `TileID.Torches` = 4。
    pub const TORCH: Self = Self(4);
    /// 自然树。正版 `TileID.Trees` = 5。
    pub const TREES: Self = Self(5);
    /// [`Self::TREES`] 的别名。
    pub const TREE: Self = Self::TREES;
    /// 铁矿。正版 `TileID.Iron` = 6。
    pub const IRON_ORE: Self = Self(6);
    /// 铜矿。正版 `TileID.Copper` = 7。
    pub const COPPER_ORE: Self = Self(7);
    /// 熔炉。正版 `TileID.Furnaces` = 17。
    pub const FURNACE: Self = Self(17);
    /// 工作台。正版 `TileID.WorkBenches` = 18。
    pub const WORKBENCH: Self = Self(18);
    /// 平台。正版 `TileID.Platforms` = 19。
    pub const PLATFORM: Self = Self(19);
    /// 树苗。正版 `TileID.Saplings` = 20。
    pub const SAPLING: Self = Self(20);
    /// 箱子。正版 `TileID.Containers` = 21。
    pub const CHEST: Self = Self(21);
    /// 木块。正版 `TileID.WoodBlock` = 30。
    pub const WOOD: Self = Self(30);
    /// 沙。正版 `TileID.Sand` = 53。
    pub const SAND: Self = Self(53);
    /// 床。正版 `TileID.Beds` = 79。
    pub const BED: Self = Self(79);
    /// 雪块。正版 `TileID.SnowBlock` = 147。
    pub const SNOW: Self = Self(147);
    /// 叶块。正版 `TileID.LeafBlock` = 192。当前生成不再铺假树冠。
    pub const LEAF: Self = Self(192);
    /// 绳索。正版 `TileID.Rope` = 213。
    pub const ROPE: Self = Self(213);
    /// 液体占格。不是物块类型。液量以后写在 `Tile` 上。
    pub const WATER: Self = Self(u32::MAX - 1);
    /// 木梯。正版没有对应 Tile。临时用本重写本地编号，不占用 0..=693 正版空间。
    pub const LADDER: Self = Self(1000);

    /// 旧开发快照里的夹具编号迁到当前身份。新世界不要再写这些旧号。
    pub fn from_fixture_id(v: u32) -> Self {
        match v {
            0 => Self::AIR,
            1 => Self::DIRT,
            2 => Self::GRASS,
            3 => Self::STONE,
            5 | 23 => Self::TREES,
            6 => Self::WOOD,
            7 => Self::LEAF,
            8 => Self::WORKBENCH,
            9 => Self::SAPLING,
            11 => Self::TORCH,
            12 => Self::PLATFORM,
            13 => Self::CHEST,
            14 => Self::LADDER,
            15 => Self::SAND,
            16 => Self::SNOW,
            17 => Self::COPPER_ORE,
            18 => Self::IRON_ORE,
            19 => Self::FURNACE,
            20 => Self::BED,
            21 => Self::WATER,
            22 => Self::ROPE,
            other => Self(other),
        }
    }
}

impl BlockId {
    pub fn solid(self) -> bool {
        if let Some(sets) = try_tile_sets() {
            return sets.solid(self.0).unwrap_or(false);
        }
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.solid)
    }

    pub fn blocks_motion(self) -> bool {
        if let Some(sets) = try_tile_sets() {
            let solid = sets.solid(self.0).unwrap_or(false);
            let top = sets.solid_top(self.0).unwrap_or(false);
            return solid || top;
        }
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.blocks_motion)
    }

    pub fn is_fluid(self) -> bool {
        // 水不是物块类型，不能放进按 id 索引的登记表。
        self == Self::WATER
            || try_content()
                .and_then(|c| c.block(self))
                .is_some_and(|d| d.is_fluid)
    }

    /// 是否自然树干。只读内容表 `is_tree`。
    pub fn is_tree(self) -> bool {
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.is_tree)
    }

    pub fn is_ladder(self) -> bool {
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.ladder)
    }

    /// 只挡自上而下的脚，可从下方穿过，也可下穿。
    pub fn is_platform(self) -> bool {
        if let Some(sets) = try_tile_sets() {
            return sets.solid_top(self.0).unwrap_or(false);
        }
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.is_platform)
    }

    pub fn mineable(self) -> bool {
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.mineable())
    }

    /// 开采所需最低镐力；`None` 表示徒手可挖或未登记。
    pub fn mine_power_need(self) -> Option<u16> {
        try_content()
            .and_then(|c| c.block(self))
            .and_then(|d| d.mine_power_need)
    }

    /// 是否作为光源。
    pub fn emits_light(self) -> bool {
        self.light_radius() > 0
    }

    pub fn light_radius(self) -> i32 {
        try_content()
            .and_then(|c| c.block(self))
            .map(|d| d.light_radius)
            .unwrap_or(0)
    }

    /// 是否显著遮挡光照传播（平台 / 半透明叶不挡）。
    pub fn blocks_light(self) -> bool {
        self.solid() && !self.is_platform()
    }

    /// 放置时是否自带帧。未登记的 id 不是 frame-important。
    pub fn frame_important(self) -> bool {
        if let Some(sets) = try_tile_sets() {
            return sets.frame_important(self.0).unwrap_or(false);
        }
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.frame_important)
    }

    /// `Tiles_N` 文件编号。没有对应图时为 `None`。
    pub fn texture_file(self) -> Option<u32> {
        try_content()
            .and_then(|c| c.block(self))
            .and_then(|d| d.texture_file)
    }

    pub fn max_hp(self) -> u16 {
        try_content()
            .and_then(|c| c.block(self))
            .map(|d| d.max_hp)
            .unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::AIR => "空气",
            Self::WATER => "水",
            _ => "未知",
        }
    }

    /// 显示名：内容表优先。
    pub fn label(self) -> String {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.name.clone();
            }
        }
        self.name().to_string()
    }

    pub fn drop_item(self) -> Option<ItemId> {
        try_content()
            .and_then(|c| c.block(self))
            .and_then(|d| d.drop)
    }

    /// 是否计入房屋室内可通行面积。空气与液体始终可计。
    pub fn house_space(self) -> bool {
        if self == Self::AIR || self.is_fluid() {
            return true;
        }
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.house_space || d.is_tree)
    }

    /// 是否满足房屋家具要求。
    pub fn house_furniture(self) -> bool {
        try_content()
            .and_then(|c| c.block(self))
            .is_some_and(|d| d.house_furniture)
    }
}

/// 背景墙标识（不挡碰撞，挖穿前景后仍可见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallId(pub u8);

impl WallId {
    pub const NONE: Self = Self(0);
    /// 石墙。正版 `WallID.Stone` = 1。
    pub const STONE: Self = Self(1);
    /// 泥土墙。正版 `WallID.Dirt` = 2。
    pub const DIRT: Self = Self(2);
    /// 木墙。正版 `WallID.Wood` = 4。
    pub const WOOD: Self = Self(4);

    /// 旧开发快照夹具墙编号迁移。
    pub fn from_fixture_id(v: u8) -> Self {
        match v {
            0 => Self::NONE,
            1 => Self::DIRT,
            2 => Self::STONE,
            3 => Self::WOOD,
            other => Self(other),
        }
    }

    /// `Wall_N` 文件编号。未登记则为 `None`。
    pub fn texture_file(self) -> Option<u32> {
        try_content()
            .and_then(|c| c.wall(self))
            .and_then(|d| d.texture_file)
    }

    pub fn color(self) -> Option<(f32, f32, f32)> {
        match self {
            Self::DIRT => Some((0.28, 0.18, 0.12)),
            Self::STONE => Some((0.22, 0.24, 0.28)),
            Self::WOOD => Some((0.32, 0.22, 0.12)),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        if self == Self::NONE {
            "无墙"
        } else {
            "未知"
        }
    }

    /// 显示名：只读内容表。
    pub fn label(self) -> String {
        if let Some(c) = try_content() {
            if let Some(d) = c.wall(self) {
                return d.name.clone();
            }
        }
        self.name().to_string()
    }

    pub fn max_hp(self) -> u16 {
        try_content()
            .and_then(|c| c.wall(self))
            .map(|d| d.max_hp)
            .unwrap_or(0)
    }

    pub fn drop_item(self) -> Option<ItemId> {
        try_content()
            .and_then(|c| c.wall(self))
            .and_then(|d| d.drop)
    }

    pub fn mineable(self) -> bool {
        self != Self::NONE && self.max_hp() > 0
    }
}

/// 物品标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl ItemId {
    /// 泥土。正版 `ItemID.DirtBlock` = 2。
    pub const DIRT: Self = Self(2);
    /// 石头。正版 `ItemID.StoneBlock` = 3。
    pub const STONE: Self = Self(3);
    /// 木材。正版 `ItemID.Wood` = 9。
    pub const WOOD: Self = Self(9);
    /// 工作台。正版 `ItemID.WorkBench` = 36。
    pub const WORKBENCH: Self = Self(36);
    /// 橡实。正版 `ItemID.Acorn` = 27。
    pub const SAPLING: Self = Self(27);
    /// 木镐。正版没有对应物品，不占用正版编号。
    pub const WOOD_PICK: Self = Self(10001);
    /// 石镐。正版没有对应物品。
    pub const STONE_PICK: Self = Self(10002);
    /// 木剑。正版 `ItemID.WoodenSword` = 24。
    pub const WOOD_SWORD: Self = Self(24);
    /// 火把。正版 `ItemID.Torch` = 8。
    pub const TORCH: Self = Self(8);
    /// 凝胶。正版 `ItemID.Gel` = 23。
    pub const GEL: Self = Self(23);
    /// 木平台。正版 `ItemID.WoodPlatform` = 94。
    pub const PLATFORM: Self = Self(94);
    /// 木箱。正版 `ItemID.Chest` = 48。
    pub const CHEST: Self = Self(48);
    /// 木梯。正版没有对应物品。
    pub const LADDER: Self = Self(10003);
    /// 木甲。正版没有对应物品。
    pub const WOOD_ARMOR: Self = Self(10004);
    /// 木弓。正版 `ItemID.WoodenBow` = 39。
    pub const WOOD_BOW: Self = Self(39);
    /// 木箭。正版 `ItemID.WoodenArrow` = 40。
    pub const WOOD_ARROW: Self = Self(40);
    /// 凝胶法杖。正版没有对应物品。
    pub const GEL_STAFF: Self = Self(10005);
    /// 沙。正版 `ItemID.SandBlock` = 169。
    pub const SAND: Self = Self(169);
    /// 雪块。正版 `ItemID.SnowBlock` = 593。
    pub const SNOW: Self = Self(593);
    /// 铜矿。正版 `ItemID.CopperOre` = 12。
    pub const COPPER_ORE: Self = Self(12);
    /// 铁矿。正版 `ItemID.IronOre` = 11。
    pub const IRON_ORE: Self = Self(11);
    /// 铜锭。正版 `ItemID.CopperBar` = 20。
    pub const COPPER_BAR: Self = Self(20);
    /// 铁锭。正版 `ItemID.IronBar` = 22。
    pub const IRON_BAR: Self = Self(22);
    /// 熔炉。正版 `ItemID.Furnace` = 33。
    pub const FURNACE: Self = Self(33);
    /// 床。正版 `ItemID.Bed` = 224。
    pub const BED: Self = Self(224);
    /// 铜镐。正版 `ItemID.CopperPickaxe` = 3509。
    pub const COPPER_PICK: Self = Self(3509);
    /// 布袋。正版没有对应物品。
    pub const CLOTH_BAG: Self = Self(10006);
    /// 旅行包。正版没有对应物品。
    pub const TRAVEL_PACK: Self = Self(10007);
    /// 钩爪。正版 `ItemID.GrapplingHook` = 84。
    pub const GRAPPLE: Self = Self(84);
    /// 云瓶。正版 `ItemID.CloudinaBottle` = 53。
    pub const CLOUD_BOTTLE: Self = Self(53);
    /// 绳索。正版 `ItemID.Rope` = 965。
    pub const ROPE: Self = Self(965);
    /// 木锤。正版 `ItemID.WoodenHammer` = 196。
    pub const WOOD_HAMMER: Self = Self(196);
    /// 木墙。正版 `ItemID.WoodWall` = 93。
    pub const WOOD_WALL: Self = Self(93);
    /// 石墙。正版 `ItemID.StoneWall` = 26。
    pub const STONE_WALL: Self = Self(26);
    /// 铜币。正版 `ItemID.CopperCoin` = 71。
    pub const COPPER_COIN: Self = Self(71);

    pub const ALL: [Self; 35] = [
        Self::DIRT,
        Self::STONE,
        Self::WOOD,
        Self::WORKBENCH,
        Self::SAPLING,
        Self::WOOD_PICK,
        Self::STONE_PICK,
        Self::WOOD_SWORD,
        Self::TORCH,
        Self::GEL,
        Self::PLATFORM,
        Self::CHEST,
        Self::LADDER,
        Self::WOOD_ARMOR,
        Self::WOOD_BOW,
        Self::WOOD_ARROW,
        Self::GEL_STAFF,
        Self::SAND,
        Self::SNOW,
        Self::COPPER_ORE,
        Self::IRON_ORE,
        Self::COPPER_BAR,
        Self::IRON_BAR,
        Self::FURNACE,
        Self::BED,
        Self::COPPER_PICK,
        Self::CLOTH_BAG,
        Self::TRAVEL_PACK,
        Self::GRAPPLE,
        Self::CLOUD_BOTTLE,
        Self::ROPE,
        Self::WOOD_HAMMER,
        Self::WOOD_WALL,
        Self::STONE_WALL,
        Self::COPPER_COIN,
    ];

    /// 旧会话夹具物品号。`TR_DEV_SESSION_V3` 起存的是正版编号，不再走这里。
    pub fn from_fixture_id(id: u32) -> Self {
        match id {
            1 => Self::DIRT,
            3 => Self::STONE,
            6 => Self::WOOD,
            8 => Self::WORKBENCH,
            9 => Self::SAPLING,
            10 => Self::WOOD_PICK,
            11 => Self::STONE_PICK,
            12 => Self::WOOD_SWORD,
            14 => Self::TORCH,
            15 => Self::GEL,
            16 => Self::PLATFORM,
            17 => Self::CHEST,
            18 => Self::LADDER,
            19 => Self::WOOD_ARMOR,
            20 => Self::WOOD_BOW,
            21 => Self::WOOD_ARROW,
            22 => Self::GEL_STAFF,
            23 => Self::SAND,
            24 => Self::SNOW,
            25 => Self::COPPER_ORE,
            26 => Self::IRON_ORE,
            27 => Self::COPPER_BAR,
            28 => Self::IRON_BAR,
            29 => Self::FURNACE,
            30 => Self::BED,
            31 => Self::COPPER_PICK,
            32 => Self::CLOTH_BAG,
            33 => Self::TRAVEL_PACK,
            34 => Self::GRAPPLE,
            35 => Self::CLOUD_BOTTLE,
            36 => Self::ROPE,
            37 => Self::WOOD_HAMMER,
            38 => Self::WOOD_WALL,
            39 => Self::STONE_WALL,
            40 => Self::COPPER_COIN,
            other => Self(other),
        }
    }

    /// `Item_N` 文件编号。
    pub fn texture_file(self) -> Option<u32> {
        try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.texture_file)
    }

    pub fn is_grapple(self) -> bool {
        try_content()
            .and_then(|c| c.item(self))
            .is_some_and(|d| d.is_grapple)
    }

    /// 锤类：优先处理背景墙。
    pub fn is_hammer(self) -> bool {
        try_content()
            .and_then(|c| c.item(self))
            .is_some_and(|d| d.is_hammer)
    }

    /// 可装备到饰品槽的物品。
    pub fn is_accessory(self) -> bool {
        try_content()
            .and_then(|c| c.item(self))
            .is_some_and(|d| d.is_accessory)
    }

    /// 持有一件该物品时额外增加的背包格数；非背包道具为 0。
    pub fn bag_bonus_slots(self) -> u32 {
        try_content()
            .and_then(|c| c.item(self))
            .map(|d| d.bag_bonus_slots)
            .unwrap_or(0)
    }

    pub fn as_block(self) -> Option<BlockId> {
        try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.places)
    }

    /// 可铺背景墙的材料。
    pub fn as_wall(self) -> Option<WallId> {
        try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.wall)
    }

    /// 食用回复量。
    pub fn heal_amount(self) -> Option<f32> {
        try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.heal)
    }

    pub fn mine_power(self) -> Option<u16> {
        try_content()
            .and_then(|c| c.item(self))
            .and_then(|d| d.mine_power)
    }

    /// 近战基础伤害（兼容旧调用）；完整参数见 [`ItemId::weapon`]。
    pub fn melee_damage(self) -> Option<u16> {
        self.weapon().and_then(|w| {
            if w.kind == weapon::WeaponKind::Melee {
                Some(w.damage as u16)
            } else {
                None
            }
        })
    }

    pub fn is_tool(self) -> bool {
        try_content().and_then(|c| c.item(self)).is_some_and(|d| {
            d.max_durability > 0 || d.mine_power.is_some() || d.weapon.is_some()
        })
    }

    /// 工具最大耐久；非工具为 0。
    pub fn max_durability(self) -> u16 {
        try_content()
            .and_then(|c| c.item(self))
            .map(|d| d.max_durability)
            .unwrap_or(0)
    }
}

#[derive(Debug)]
pub enum TerrariaError {
    NotImplemented { feature: &'static str },
}

impl TerrariaError {
    pub fn not_implemented(feature: &'static str) -> Self {
        Self::NotImplemented { feature }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented { .. } => "tr.core.not_implemented",
        }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::NotImplemented { feature } => {
                ErrorArgs::new().with("feature", ErrorArg::String(Arc::from(*feature)))
            }
        }
    }
}

impl fmt::Display for TerrariaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for TerrariaError {}
