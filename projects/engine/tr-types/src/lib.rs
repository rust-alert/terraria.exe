#![warn(missing_docs)]
//! Terraria 内容 schema（无世界实体）。
//!
//! 本 crate 描述作者声明的**类型契约**与字段默认/覆盖合并。
//! 数值 ID、注册表与世界权威状态仍在 `tr-core`。

mod block;
mod content_path;
mod item;
mod locale_text;
mod overlay;

pub use block::{BlockTexture, TerrariaBlock};
pub use content_path::{ContentPath, ContentPathError};
pub use item::{
    TerrariaConsumableItem, TerrariaItem, TerrariaPlaceableItem, TerrariaToolItem, ToolKind,
};
pub use locale_text::{LocaleSymbol, LocaleText};
pub use overlay::{BlockFieldOverride, ItemFieldOverride, merge_block, merge_placeable_item};
