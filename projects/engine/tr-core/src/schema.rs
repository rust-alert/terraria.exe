//! `tr-types` schema → 运行时 [`BlockDef`] / [`ItemDef`]。
//!
//! 内容身份用 [`ContentPath`]；注册表里的 `key` 暂存路径字符串，供过渡期脚本/存档对照。
//! 展示名优先存 Locale 符号路径，由本地化层解析为当前语言文本。

use tr_types::{
    BlockTexture, ContentPath, LocaleText, TerrariaBlock, TerrariaConsumableItem,
    TerrariaPlaceableItem, TerrariaToolItem,
};

use crate::BlockId;
use crate::content::{BlockDef, ItemDef};

/// 将 typed 方块 schema 物化为注册表条目。
pub fn block_def_from_schema(path: &ContentPath, block: &TerrariaBlock) -> BlockDef {
    let (texture, texture_top, texture_side, texture_bottom) = texture_paths(&block.texture);
    BlockDef {
        key: path.to_string(),
        name: block.name.symbol.as_str().to_string(),
        solid: block.solid,
        blocks_motion: block.blocks_motion,
        ladder: block.ladder,
        replaceable: block.replaceable,
        max_hp: block.max_hp,
        light_radius: block.light_radius,
        mine_power_need: None,
        drop: None,
        color: [0.5, 0.5, 0.5, 1.0],
        texture,
        texture_top,
        texture_side,
        texture_bottom,
        is_tree: false,
        frame_important: false,
        texture_file: None,
        is_platform: false,
        is_fluid: false,
        framed_terrain: false,
    }
}

/// 可放置物品 schema → [`ItemDef`]。
pub fn placeable_item_from_schema(
    path: &ContentPath,
    item: &TerrariaPlaceableItem,
    places: Option<BlockId>,
) -> ItemDef {
    ItemDef {
        key: path.to_string(),
        name: item.base.name.symbol.as_str().to_string(),
        places,
        wall: None,
        heal: None,
        mine_power: None,
        max_durability: 0,
        weapon: None,
        color: [0.5, 0.5, 0.5],
        in_palette: true,
        texture: item.base.texture.to_string(),
        texture_file: None,
        bag_bonus_slots: 0,
    }
}

/// 工具物品 schema → [`ItemDef`]。
pub fn tool_item_from_schema(path: &ContentPath, item: &TerrariaToolItem) -> ItemDef {
    ItemDef {
        key: path.to_string(),
        name: item.base.name.symbol.as_str().to_string(),
        places: None,
        wall: None,
        heal: None,
        mine_power: Some(item.mine_power),
        max_durability: item.max_durability,
        weapon: None,
        color: [0.5, 0.5, 0.5],
        in_palette: true,
        texture: item.base.texture.to_string(),
        texture_file: None,
        bag_bonus_slots: 0,
    }
}

/// 消耗品 schema → [`ItemDef`]。
pub fn consumable_item_from_schema(path: &ContentPath, item: &TerrariaConsumableItem) -> ItemDef {
    ItemDef {
        key: path.to_string(),
        name: item.base.name.symbol.as_str().to_string(),
        places: None,
        wall: None,
        heal: Some(item.heal),
        mine_power: None,
        max_durability: 0,
        weapon: None,
        color: [0.5, 0.5, 0.5],
        in_palette: true,
        texture: item.base.texture.to_string(),
        texture_file: None,
        bag_bonus_slots: 0,
    }
}

fn texture_paths(tex: &BlockTexture) -> (String, String, String, String) {
    (
        tex.default.to_string(),
        tex.top.as_ref().map(|s| s.to_string()).unwrap_or_default(),
        tex.side.as_ref().map(|s| s.to_string()).unwrap_or_default(),
        tex.bottom
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default(),
    )
}

/// 便捷：泥土块声明语义（仅覆盖 name）。
pub fn example_dirt_block() -> (ContentPath, TerrariaBlock) {
    use tr_types::{BlockFieldOverride, merge_block};
    let path = ContentPath::parse("terraria::blocks::Dirt").expect("path");
    let block = merge_block(
        &TerrariaBlock::default(),
        &BlockFieldOverride {
            name: Some(LocaleText::new("standard.block.dirt")),
            ..Default::default()
        },
    );
    (path, block)
}

/// [`tr_types::ToolKind`] 透传，避免游戏层重复定义。
pub use tr_types::ToolKind as SchemaToolKind;
