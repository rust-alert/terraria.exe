#![warn(missing_docs)]
//! Terraria 游戏层基类型、方块与物品表。

mod biome;
mod content;
mod damage;
mod fluid;
mod schema;
mod weapon;

pub use biome::{BiomeId, biome_at};
pub use content::{
    BlockDef, BlockFaceKind, ContentRegistry, ItemDef, block_def, content, install,
    install_builtin_fixture, is_installed, item_def, try_content,
};
pub use damage::{DamageHit, DamageType, ResistProfile, resolve_damage};
pub use fluid::FluidLevel;
pub use schema::{
    SchemaToolKind, block_def_from_schema, consumable_item_from_schema, example_dirt_block,
    placeable_item_from_schema, tool_item_from_schema,
};
pub use weapon::{WeaponKind, WeaponStats};

use std::fmt;
use std::sync::Arc;

use spark_core::{ErrorArg, ErrorArgs};

/// 方块标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl BlockId {
    pub const AIR: Self = Self(0);
    pub const DIRT: Self = Self(1);
    pub const GRASS: Self = Self(2);
    pub const STONE: Self = Self(3);
    pub const SCRAP: Self = Self(4);
    pub const POD: Self = Self(5);
    pub const WOOD: Self = Self(6);
    pub const LEAF: Self = Self(7);
    pub const WORKBENCH: Self = Self(8);
    pub const SAPLING: Self = Self(9);
    /// 传送节点（裂痕锚）。
    pub const WARP: Self = Self(10);
    /// 火把（可穿行，夜间照明）。
    pub const TORCH: Self = Self(11);
    /// 木平台（可站立，便于架空建造）。
    pub const PLATFORM: Self = Self(12);
    /// 木箱（可存放物品）。
    pub const CHEST: Self = Self(13);
    /// 木梯（可穿行，可攀爬）。
    pub const LADDER: Self = Self(14);
    /// 沙（荒原地表）。
    pub const SAND: Self = Self(15);
    /// 雪（寒地地表）。
    pub const SNOW: Self = Self(16);
    /// 铜矿。
    pub const COPPER_ORE: Self = Self(17);
    /// 铁矿。
    pub const IRON_ORE: Self = Self(18);
    /// 熔炉。
    pub const FURNACE: Self = Self(19);
    /// 床（家园重生点）。
    pub const BED: Self = Self(20);
    /// 水（Minecraft 风格流动液体；水位见 [`FluidLevel`]）。
    pub const WATER: Self = Self(21);
    /// 绳索（可穿行，可攀爬，便于竖井）。
    pub const ROPE: Self = Self(22);
}

impl BlockId {
    pub fn solid(self) -> bool {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.solid;
            }
        }
        !matches!(
            self,
            Self::AIR
                | Self::LEAF
                | Self::SAPLING
                | Self::TORCH
                | Self::LADDER
                | Self::ROPE
                | Self::WATER
        )
    }

    pub fn blocks_motion(self) -> bool {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.blocks_motion;
            }
        }
        !matches!(
            self,
            Self::AIR
                | Self::LEAF
                | Self::SAPLING
                | Self::WARP
                | Self::TORCH
                | Self::LADDER
                | Self::ROPE
                | Self::WATER
        )
    }

    pub fn is_fluid(self) -> bool {
        self == Self::WATER
    }

    pub fn is_ladder(self) -> bool {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.ladder;
            }
        }
        self == Self::LADDER || self == Self::ROPE
    }

    /// 只挡自上而下的脚，可从下方穿过，也可下穿。
    pub fn is_platform(self) -> bool {
        self == Self::PLATFORM
    }

    pub fn mineable(self) -> bool {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.mineable() && self != Self::POD;
            }
        }
        !matches!(self, Self::AIR | Self::POD | Self::WATER) && self.max_hp() < u16::MAX
    }

    /// 开采所需最低镐力；`None` 表示徒手可挖。
    pub fn mine_power_need(self) -> Option<u16> {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.mine_power_need;
            }
        }
        match self {
            Self::COPPER_ORE => Some(30),
            Self::IRON_ORE => Some(35),
            _ => None,
        }
    }

    /// 是否作为光源。
    pub fn emits_light(self) -> bool {
        self.light_radius() > 0
    }

    pub fn light_radius(self) -> i32 {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.light_radius;
            }
        }
        match self {
            Self::TORCH => 8,
            Self::POD => 6,
            Self::WARP => 4,
            Self::FURNACE => 5,
            _ => 0,
        }
    }

    /// 是否显著遮挡光照传播（平台 / 半透明叶不挡）。
    pub fn blocks_light(self) -> bool {
        self.solid() && !matches!(self, Self::PLATFORM)
    }

    pub fn max_hp(self) -> u16 {
        if let Some(c) = try_content() {
            if let Some(d) = c.block(self) {
                return d.max_hp;
            }
        }
        match self {
            Self::AIR | Self::WATER => 0,
            Self::SAPLING | Self::TORCH | Self::LADDER | Self::ROPE => 10,
            Self::LEAF => 20,
            Self::PLATFORM | Self::BED => 40,
            Self::DIRT | Self::GRASS | Self::SAND | Self::SNOW => 50,
            Self::WOOD | Self::CHEST => 60,
            Self::SCRAP | Self::COPPER_ORE => 80,
            Self::STONE | Self::WORKBENCH | Self::FURNACE => 100,
            Self::IRON_ORE | Self::WARP => 150,
            Self::POD => u16::MAX,
            _ => 100,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::AIR => "空气",
            Self::DIRT => "泥土",
            Self::GRASS => "草皮",
            Self::STONE => "石头",
            Self::SCRAP => "废料",
            Self::POD => "逃生舱",
            Self::WOOD => "木材",
            Self::LEAF => "树叶",
            Self::WORKBENCH => "工作台",
            Self::SAPLING => "树苗",
            Self::WARP => "裂痕锚",
            Self::TORCH => "火把",
            Self::PLATFORM => "木平台",
            Self::CHEST => "木箱",
            Self::LADDER => "木梯",
            Self::SAND => "沙子",
            Self::SNOW => "雪块",
            Self::COPPER_ORE => "铜矿",
            Self::IRON_ORE => "铁矿",
            Self::FURNACE => "熔炉",
            Self::BED => "床",
            Self::WATER => "水",
            Self::ROPE => "绳索",
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
        match self {
            Self::DIRT | Self::GRASS => Some(ItemId::DIRT),
            Self::STONE => Some(ItemId::STONE),
            Self::SCRAP => Some(ItemId::SCRAP),
            Self::WOOD => Some(ItemId::WOOD),
            Self::WORKBENCH => Some(ItemId::WORKBENCH),
            Self::SAPLING => Some(ItemId::SAPLING),
            Self::WARP => Some(ItemId::WARP),
            Self::TORCH => Some(ItemId::TORCH),
            Self::PLATFORM => Some(ItemId::PLATFORM),
            Self::CHEST => Some(ItemId::CHEST),
            Self::LADDER => Some(ItemId::LADDER),
            Self::SAND => Some(ItemId::SAND),
            Self::SNOW => Some(ItemId::SNOW),
            Self::COPPER_ORE => Some(ItemId::COPPER_ORE),
            Self::IRON_ORE => Some(ItemId::IRON_ORE),
            Self::FURNACE => Some(ItemId::FURNACE),
            Self::BED => Some(ItemId::BED),
            Self::ROPE => Some(ItemId::ROPE),
            Self::LEAF | Self::AIR | Self::POD | Self::WATER => None,
            _ => None,
        }
    }
}

/// 背景墙标识（不挡碰撞，挖穿前景后仍可见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallId(pub u8);

impl WallId {
    pub const NONE: Self = Self(0);
    pub const DIRT: Self = Self(1);
    pub const STONE: Self = Self(2);
    pub const WOOD: Self = Self(3);

    pub fn color(self) -> Option<(f32, f32, f32)> {
        match self {
            Self::DIRT => Some((0.28, 0.18, 0.12)),
            Self::STONE => Some((0.22, 0.24, 0.28)),
            Self::WOOD => Some((0.32, 0.22, 0.12)),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::DIRT => "泥土墙",
            Self::STONE => "石墙",
            Self::WOOD => "木墙",
            _ => "无墙",
        }
    }

    /// 显示名：内容表优先（当前墙无独立内容项，回退常量名）。
    pub fn label(self) -> String {
        self.name().to_string()
    }

    pub fn max_hp(self) -> u16 {
        match self {
            Self::NONE => 0,
            Self::DIRT => 40,
            Self::STONE => 70,
            Self::WOOD => 50,
            _ => 40,
        }
    }

    pub fn drop_item(self) -> Option<ItemId> {
        match self {
            Self::DIRT => Some(ItemId::DIRT),
            Self::STONE => Some(ItemId::STONE),
            Self::WOOD => Some(ItemId::WOOD),
            _ => None,
        }
    }

    pub fn mineable(self) -> bool {
        self != Self::NONE && self.max_hp() > 0
    }
}

/// 物品标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl ItemId {
    pub const DIRT: Self = Self(1);
    pub const STONE: Self = Self(3);
    pub const SCRAP: Self = Self(4);
    pub const WOOD: Self = Self(6);
    pub const WORKBENCH: Self = Self(8);
    pub const SAPLING: Self = Self(9);
    pub const WOOD_PICK: Self = Self(10);
    pub const STONE_PICK: Self = Self(11);
    pub const WOOD_SWORD: Self = Self(12);
    pub const WARP: Self = Self(13);
    pub const TORCH: Self = Self(14);
    pub const GEL: Self = Self(15);
    pub const PLATFORM: Self = Self(16);
    pub const CHEST: Self = Self(17);
    pub const LADDER: Self = Self(18);
    pub const WOOD_ARMOR: Self = Self(19);
    pub const WOOD_BOW: Self = Self(20);
    pub const WOOD_ARROW: Self = Self(21);
    pub const GEL_STAFF: Self = Self(22);
    pub const SAND: Self = Self(23);
    pub const SNOW: Self = Self(24);
    pub const COPPER_ORE: Self = Self(25);
    pub const IRON_ORE: Self = Self(26);
    pub const COPPER_BAR: Self = Self(27);
    pub const IRON_BAR: Self = Self(28);
    pub const FURNACE: Self = Self(29);
    pub const BED: Self = Self(30);
    pub const COPPER_PICK: Self = Self(31);
    /// 布袋：持有时增加背包格。
    pub const CLOTH_BAG: Self = Self(32);
    /// 旅行包：更大容量加成。
    pub const TRAVEL_PACK: Self = Self(33);
    /// 钩爪：抛出挂墙牵引移动。
    pub const GRAPPLE: Self = Self(34);
    /// 凝胶云瓶：装备后可二段跳。
    pub const CLOUD_BOTTLE: Self = Self(35);
    /// 绳索：可放置攀爬。
    pub const ROPE: Self = Self(36);
    /// 木锤：拆除背景墙。
    pub const WOOD_HAMMER: Self = Self(37);

    pub const ALL: [Self; 34] = [
        Self::DIRT,
        Self::STONE,
        Self::SCRAP,
        Self::WOOD,
        Self::WORKBENCH,
        Self::SAPLING,
        Self::WOOD_PICK,
        Self::STONE_PICK,
        Self::WOOD_SWORD,
        Self::WARP,
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
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::DIRT => "泥土",
            Self::STONE => "石头",
            Self::SCRAP => "废料",
            Self::WOOD => "木材",
            Self::WORKBENCH => "工作台",
            Self::SAPLING => "树苗",
            Self::WOOD_PICK => "木镐",
            Self::STONE_PICK => "石镐",
            Self::WOOD_SWORD => "木剑",
            Self::WARP => "裂痕锚",
            Self::TORCH => "火把",
            Self::GEL => "凝胶",
            Self::PLATFORM => "木平台",
            Self::CHEST => "木箱",
            Self::LADDER => "木梯",
            Self::WOOD_ARMOR => "树脂甲",
            Self::WOOD_BOW => "木弓",
            Self::WOOD_ARROW => "木箭",
            Self::GEL_STAFF => "凝胶法杖",
            Self::SAND => "沙子",
            Self::SNOW => "雪块",
            Self::COPPER_ORE => "铜矿",
            Self::IRON_ORE => "铁矿",
            Self::COPPER_BAR => "铜锭",
            Self::IRON_BAR => "铁锭",
            Self::FURNACE => "熔炉",
            Self::BED => "床",
            Self::COPPER_PICK => "铜镐",
            Self::CLOTH_BAG => "布袋",
            Self::TRAVEL_PACK => "旅行包",
            Self::GRAPPLE => "钩爪",
            Self::CLOUD_BOTTLE => "凝胶云瓶",
            Self::ROPE => "绳索",
            Self::WOOD_HAMMER => "木锤",
            _ => "未知",
        }
    }

    pub fn is_grapple(self) -> bool {
        self == Self::GRAPPLE
    }

    /// 锤类：优先处理背景墙。
    pub fn is_hammer(self) -> bool {
        self == Self::WOOD_HAMMER
    }

    /// 可装备到饰品槽的物品。
    pub fn is_accessory(self) -> bool {
        matches!(self, Self::CLOUD_BOTTLE)
    }

    /// 持有一件该物品时额外增加的背包格数；非背包道具为 0。
    pub fn bag_bonus_slots(self) -> u32 {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.bag_bonus_slots;
            }
        }
        match self {
            Self::CLOTH_BAG => 8,
            Self::TRAVEL_PACK => 16,
            _ => 0,
        }
    }

    pub fn as_block(self) -> Option<BlockId> {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.places;
            }
        }
        match self {
            Self::DIRT => Some(BlockId::DIRT),
            Self::STONE => Some(BlockId::STONE),
            Self::SCRAP => Some(BlockId::SCRAP),
            Self::WOOD => Some(BlockId::WOOD),
            Self::WORKBENCH => Some(BlockId::WORKBENCH),
            Self::SAPLING => Some(BlockId::SAPLING),
            Self::WARP => Some(BlockId::WARP),
            Self::TORCH => Some(BlockId::TORCH),
            Self::PLATFORM => Some(BlockId::PLATFORM),
            Self::CHEST => Some(BlockId::CHEST),
            Self::LADDER => Some(BlockId::LADDER),
            Self::ROPE => Some(BlockId::ROPE),
            Self::SAND => Some(BlockId::SAND),
            Self::SNOW => Some(BlockId::SNOW),
            Self::COPPER_ORE => Some(BlockId::COPPER_ORE),
            Self::IRON_ORE => Some(BlockId::IRON_ORE),
            Self::FURNACE => Some(BlockId::FURNACE),
            Self::BED => Some(BlockId::BED),
            _ => None,
        }
    }

    /// 可铺背景墙的材料。
    pub fn as_wall(self) -> Option<WallId> {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.wall;
            }
        }
        match self {
            Self::DIRT => Some(WallId::DIRT),
            Self::STONE => Some(WallId::STONE),
            Self::WOOD => Some(WallId::WOOD),
            _ => None,
        }
    }

    /// 食用回复量。
    pub fn heal_amount(self) -> Option<f32> {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.heal;
            }
        }
        match self {
            Self::GEL => Some(18.0),
            _ => None,
        }
    }

    pub fn mine_power(self) -> Option<u16> {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.mine_power;
            }
        }
        match self {
            Self::WOOD_PICK => Some(25),
            Self::COPPER_PICK => Some(35),
            Self::STONE_PICK => Some(40),
            _ => None,
        }
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
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.max_durability > 0 || d.mine_power.is_some() || d.weapon.is_some();
            }
        }
        matches!(
            self,
            Self::WOOD_PICK
                | Self::STONE_PICK
                | Self::COPPER_PICK
                | Self::WOOD_HAMMER
                | Self::WOOD_SWORD
                | Self::WOOD_BOW
                | Self::GEL_STAFF
        )
    }

    /// 工具最大耐久；非工具为 0。
    pub fn max_durability(self) -> u16 {
        if let Some(c) = try_content() {
            if let Some(d) = c.item(self) {
                return d.max_durability;
            }
        }
        match self {
            Self::WOOD_PICK => 80,
            Self::COPPER_PICK => 140,
            Self::STONE_PICK => 180,
            Self::WOOD_HAMMER => 100,
            Self::WOOD_SWORD => 100,
            Self::WOOD_BOW => 120,
            Self::GEL_STAFF => 90,
            _ => 0,
        }
    }
}

/// 种族标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RaceId(pub u32);

impl RaceId {
    pub const HUMAN: Self = Self(0);
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
