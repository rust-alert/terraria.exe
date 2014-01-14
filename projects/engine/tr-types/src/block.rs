//! 方块内容 schema（类型契约，非世界实体）。

use std::sync::Arc;

use crate::locale_text::LocaleText;

/// 方块三面贴图引用（逻辑资源符号，非本机路径）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockTexture {
    pub default: Arc<str>,
    pub top: Option<Arc<str>>,
    pub side: Option<Arc<str>>,
    pub bottom: Option<Arc<str>>,
}

impl BlockTexture {
    pub fn uniform(path: impl Into<Arc<str>>) -> Self {
        Self {
            default: path.into(),
            top: None,
            side: None,
            bottom: None,
        }
    }

    pub fn faces(
        default: impl Into<Arc<str>>,
        top: impl Into<Arc<str>>,
        side: impl Into<Arc<str>>,
        bottom: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            default: default.into(),
            top: Some(top.into()),
            side: Some(side.into()),
            bottom: Some(bottom.into()),
        }
    }
}

impl Default for BlockTexture {
    fn default() -> Self {
        Self::uniform("asset.textures.default_block")
    }
}

/// `TerrariaBlock` 内容类型契约与默认字段。
///
/// 对应作者声明：
/// ```text
/// class TerrariaBlock {
///     name: LocaleText
///     texture: BlockTexture
///     solid: Bool = true
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TerrariaBlock {
    pub name: LocaleText,
    pub texture: BlockTexture,
    pub solid: bool,
    pub blocks_motion: bool,
    pub max_hp: u16,
    pub light_radius: i32,
    pub ladder: bool,
    pub replaceable: bool,
}

impl Default for TerrariaBlock {
    fn default() -> Self {
        Self {
            name: LocaleText::new("standard.block.unnamed"),
            texture: BlockTexture::default(),
            solid: true,
            blocks_motion: true,
            max_hp: 50,
            light_radius: 0,
            ladder: false,
            replaceable: false,
        }
    }
}
