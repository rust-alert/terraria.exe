//! 内容贴图路径表。
//!
//! 不加载脚本，也不走自建内容包。原版 mod 加载固定为 [`MOD_LOAD_STATUS`]。

use std::path::{Path, PathBuf};

use spark_image::PixelImage;
use tr_core::{BlockId, ItemId, WallId};

/// 原版 mod 加载状态字面量。当前实现只允许这个值。
pub const MOD_LOAD_STATUS: &str = "unsupported";

/// 原版 mod 加载。现在没有实现，禁止用脚本或自建内容包代替。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModLoad {
    Unsupported,
}

impl ModLoad {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => MOD_LOAD_STATUS,
        }
    }
}

/// 原版 mod 加载入口。直接返回 [`ModLoad::Unsupported`]。
pub fn load_original_mods(_install: &Path) -> ModLoad {
    ModLoad::Unsupported
}

/// 启动内容。原版 mod 仍为 unsupported；过渡期仅装内置方块表，贴图路径为空。
pub fn boot_content(install: &Path) -> ContentAssets {
    let status = load_original_mods(install);
    tracing::warn!(
        target: "tr.content",
        path = %install.display(),
        status = status.as_str(),
        "原版 mod 加载尚未实现"
    );
    tr_core::install_builtin_fixture();
    ContentAssets::empty()
}

/// 已解析的贴图路径，供图集构建。当前为空。
#[derive(Debug, Default, Clone)]
pub struct ContentAssets {
    pub block_side: std::collections::HashMap<BlockId, PathBuf>,
    pub block_top: std::collections::HashMap<BlockId, PathBuf>,
    pub item_icon: std::collections::HashMap<ItemId, PathBuf>,
    pub wall_tex: std::collections::HashMap<WallId, PathBuf>,
    pub sky_backdrop: Option<PathBuf>,
    pub sky_stars: Option<PathBuf>,
    pub sky_body: Option<PathBuf>,
    pub sky_clouds: Option<PathBuf>,
    pub sky_hills_meadow: Option<PathBuf>,
    pub sky_hills_forest: Option<PathBuf>,
    pub sky_hills_desert: Option<PathBuf>,
    pub sky_hills_tundra: Option<PathBuf>,
}

impl ContentAssets {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load_rgba(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
        let img = PixelImage::load(path).ok()?;
        Some((img.width(), img.height(), img.into_rgba()))
    }
}
