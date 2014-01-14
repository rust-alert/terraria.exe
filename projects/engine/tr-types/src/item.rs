//! 物品内容 schema。

use std::sync::Arc;

use crate::content_path::ContentPath;
use crate::locale_text::LocaleText;

/// 工具种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Hammer,
    Shovel,
}

/// 通用物品基类型。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrariaItem {
    pub name: LocaleText,
    pub texture: Arc<str>,
    pub max_stack: u16,
}

impl Default for TerrariaItem {
    fn default() -> Self {
        Self {
            name: LocaleText::new("standard.item.unnamed"),
            texture: Arc::from("asset.textures.default_item"),
            max_stack: 99,
        }
    }
}

/// 可放置物品。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrariaPlaceableItem {
    pub base: TerrariaItem,
    pub places: Option<ContentPath>,
}

impl Default for TerrariaPlaceableItem {
    fn default() -> Self {
        Self {
            base: TerrariaItem::default(),
            places: None,
        }
    }
}

/// 工具物品。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrariaToolItem {
    pub base: TerrariaItem,
    pub tool_kind: ToolKind,
    pub mine_power: u16,
    pub max_durability: u16,
}

impl Default for TerrariaToolItem {
    fn default() -> Self {
        Self {
            base: TerrariaItem {
                max_stack: 1,
                ..TerrariaItem::default()
            },
            tool_kind: ToolKind::Pickaxe,
            mine_power: 1,
            max_durability: 100,
        }
    }
}

/// 消耗品。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrariaConsumableItem {
    pub base: TerrariaItem,
    pub heal: f32,
}

impl Default for TerrariaConsumableItem {
    fn default() -> Self {
        Self {
            base: TerrariaItem {
                max_stack: 30,
                ..TerrariaItem::default()
            },
            heal: 0.0,
        }
    }
}
