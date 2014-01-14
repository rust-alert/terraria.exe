//! 字段覆盖合并：子类只写差异，其余继承父默认。

use crate::block::{BlockTexture, TerrariaBlock};
use crate::content_path::ContentPath;
use crate::item::{TerrariaItem, TerrariaPlaceableItem};
use crate::locale_text::LocaleText;

/// 方块字段覆盖（对应 class 体中写出的字段）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockFieldOverride {
    pub name: Option<LocaleText>,
    pub texture: Option<BlockTexture>,
    pub solid: Option<bool>,
    pub blocks_motion: Option<bool>,
    pub max_hp: Option<u16>,
    pub light_radius: Option<i32>,
    pub ladder: Option<bool>,
    pub replaceable: Option<bool>,
}

/// 将覆盖合并到父类型默认值。
pub fn merge_block(base: &TerrariaBlock, over: &BlockFieldOverride) -> TerrariaBlock {
    TerrariaBlock {
        name: over.name.clone().unwrap_or_else(|| base.name.clone()),
        texture: over.texture.clone().unwrap_or_else(|| base.texture.clone()),
        solid: over.solid.unwrap_or(base.solid),
        blocks_motion: over.blocks_motion.unwrap_or(base.blocks_motion),
        max_hp: over.max_hp.unwrap_or(base.max_hp),
        light_radius: over.light_radius.unwrap_or(base.light_radius),
        ladder: over.ladder.unwrap_or(base.ladder),
        replaceable: over.replaceable.unwrap_or(base.replaceable),
    }
}

/// 可放置物品字段覆盖。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemFieldOverride {
    pub name: Option<LocaleText>,
    pub texture: Option<std::sync::Arc<str>>,
    pub max_stack: Option<u16>,
    pub places: Option<ContentPath>,
}

pub fn merge_placeable_item(
    base: &TerrariaPlaceableItem,
    over: &ItemFieldOverride,
) -> TerrariaPlaceableItem {
    TerrariaPlaceableItem {
        base: TerrariaItem {
            name: over.name.clone().unwrap_or_else(|| base.base.name.clone()),
            texture: over
                .texture
                .clone()
                .unwrap_or_else(|| base.base.texture.clone()),
            max_stack: over.max_stack.unwrap_or(base.base.max_stack),
        },
        places: over.places.clone().or_else(|| base.places.clone()),
    }
}
