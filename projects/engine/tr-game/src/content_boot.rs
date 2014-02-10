//! 内容索引与启动。
//!
//! 正版安装只提供 XNB。夹具方块表不是正版编号，禁止用夹具 ID 去打开 `Tiles_N.xnb`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use spark_image::PixelImage;
use tr_core::{BlockId, ItemId, WallId};

use crate::xnb::{RgbaTexture, decode_texture_file};

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

/// `Content/Images` 顶层按文件名索引。不解码，也不按夹具 ID 对齐。
#[derive(Debug, Clone, Default)]
pub struct ImageIndex {
    pub items: HashMap<u32, PathBuf>,
    pub tiles: HashMap<u32, PathBuf>,
    pub walls: HashMap<u32, PathBuf>,
    pub npcs: HashMap<u32, PathBuf>,
    pub players: Vec<PathBuf>,
}

/// 扫描 `Images` 目录。缺目录即失败。
pub fn index_images(images: &Path) -> Result<ImageIndex, String> {
    if !images.is_dir() {
        return Err(format!(
            "未找到贴图目录：{}。必须是正版安装里的 Content/Images。",
            images.display()
        ));
    }
    let mut index = ImageIndex::default();
    let rd = std::fs::read_dir(images).map_err(|e| format!("无法列出贴图目录：{e}"))?;
    for ent in rd {
        let ent = ent.map_err(|e| format!("读取贴图目录项失败：{e}"))?;
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(id) = numbered_id(name, "Item_") {
            index.items.insert(id, path);
        } else if let Some(id) = numbered_id(name, "Tiles_") {
            index.tiles.insert(id, path);
        } else if let Some(id) = numbered_id(name, "Wall_") {
            index.walls.insert(id, path);
        } else if let Some(id) = numbered_id(name, "NPC_") {
            index.npcs.insert(id, path);
        } else if name.starts_with("Player_") && name.ends_with(".xnb") {
            index.players.push(path);
        }
    }
    index.players.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .cmp(b.file_name().unwrap_or_default())
    });
    Ok(index)
}

fn numbered_id(file_name: &str, prefix: &str) -> Option<u32> {
    let rest = file_name.strip_prefix(prefix)?.strip_suffix(".xnb")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn decode_proof(index: &ImageIndex) -> Result<Vec<RgbaTexture>, String> {
    let tiles = index
        .tiles
        .get(&0)
        .ok_or("缺少 Tiles_0.xnb。无法证明正版贴图可读。")?;
    let item_id = index
        .items
        .keys()
        .copied()
        .min()
        .ok_or("缺少 Item_N.xnb。无法证明正版物品贴图可读。")?;
    let item = &index.items[&item_id];
    let player = index
        .players
        .iter()
        .find(|p| p.file_name().and_then(|s| s.to_str()) == Some("Player_0_0.xnb"))
        .or_else(|| index.players.first())
        .ok_or("缺少 Player_*.xnb。无法证明正版玩家条带可读。")?;

    let mut out = Vec::with_capacity(3);
    for path in [tiles, item, player] {
        out.push(decode_texture_file(path)?);
    }
    Ok(out)
}

fn log_pixel_grid(tex: &RgbaTexture) {
    let opaque = tex.rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
    if tex.width % 16 == 0 && tex.height % 16 == 0 {
        tracing::info!(
            target: "tr.content",
            name = %tex.name,
            w = tex.width,
            h = tex.height,
            cols = tex.width / 16,
            rows = tex.height / 16,
            opaque,
            "像素边长可被 16 整除。切片以这张图为准，不套用外部帧表"
        );
    } else {
        tracing::info!(
            target: "tr.content",
            name = %tex.name,
            w = tex.width,
            h = tex.height,
            opaque,
            "像素边长不能被 16 整除。不按 16 格硬切"
        );
    }
}

/// 只登记玩法模块，不读安装目录。查询物块属性前必须先走到这里。
pub fn register_play_content() {
    tr_core::boot_content_modules(&[
        &crate::content_tiles::VanillaTiles,
        &crate::content_items::VanillaWalls,
        &crate::content_items::BootstrapItems,
        &crate::content_recipes::BootstrapRecipes,
    ])
    .expect("内容图启动");
}

/// 启动内容。解码证明贴图失败则返回错误，不退回程序化色块冒充正版素材。
pub fn boot_content(install: &Path) -> Result<ContentAssets, String> {
    let images = install.join("Content").join("Images");
    let index = index_images(&images)?;
    let proof = decode_proof(&index)?;
    for tex in &proof {
        log_pixel_grid(tex);
    }
    tracing::info!(
        target: "tr.content",
        items = index.items.len(),
        tiles = index.tiles.len(),
        walls = index.walls.len(),
        npcs = index.npcs.len(),
        players = index.players.len(),
        "已索引正版 XNB"
    );
    let status = load_original_mods(install);
    tracing::info!(
        target: "tr.content",
        path = %install.display(),
        status = status.as_str(),
        "mod 包未提供。贴图索引与模组状态无关"
    );
    tracing::info!(
        target: "tr.content",
        "内容图由 ContentModule 注册。物块使用公开类型 id，配方走同一注册表"
    );
    register_play_content();
    let mut assets = ContentAssets::empty();
    assets.install_root = Some(install.to_path_buf());
    assets.boot_frames = proof;
    assets.tile_sheets = index.tiles;
    assets.item_sheets = index.items;
    assets.wall_sheets = index.walls;
    assets.npc_sheets = index.npcs;
    assets.player_sheets = index.players;
    let backdrop = images.join("Background_0.xnb");
    if backdrop.is_file() {
        assets.forest_background = Some(backdrop);
    }
    for (field, name) in [
        (&mut assets.hud_heart, "Heart.xnb"),
        (&mut assets.hud_mana, "Mana.xnb"),
        (&mut assets.hud_inv_back, "Inventory_Back.xnb"),
    ] {
        let p = images.join(name);
        if p.is_file() {
            *field = Some(p);
        } else {
            tracing::warn!(name, "缺少 HUD 铬件贴图");
        }
    }
    Ok(assets)
}

/// 已解析的贴图。PNG 字段留给夹具，正版对照在 [`ContentAssets::boot_frames`]。
#[derive(Debug, Default, Clone)]
pub struct ContentAssets {
    pub block_side: HashMap<BlockId, PathBuf>,
    pub block_top: HashMap<BlockId, PathBuf>,
    pub item_icon: HashMap<ItemId, PathBuf>,
    pub wall_tex: HashMap<WallId, PathBuf>,
    pub sky_backdrop: Option<PathBuf>,
    pub sky_stars: Option<PathBuf>,
    pub sky_body: Option<PathBuf>,
    pub sky_clouds: Option<PathBuf>,
    pub sky_hills_meadow: Option<PathBuf>,
    pub sky_hills_forest: Option<PathBuf>,
    pub sky_hills_desert: Option<PathBuf>,
    pub sky_hills_tundra: Option<PathBuf>,
    /// 启动时解码的少数正版贴图。不按夹具 ID 绑定。
    pub boot_frames: Vec<RgbaTexture>,
    /// `Tiles_{id}.xnb` 路径。键是文件名编号，不是夹具方块 ID。
    pub tile_sheets: HashMap<u32, PathBuf>,
    /// `Item_{id}.xnb` 路径。键是文件名编号，不是夹具物品 ID。
    pub item_sheets: HashMap<u32, PathBuf>,
    /// `Wall_{id}.xnb` 路径。键是文件名编号，不是夹具墙 ID。
    pub wall_sheets: HashMap<u32, PathBuf>,
    /// `NPC_{id}.xnb` 路径。键是文件名编号。
    pub npc_sheets: HashMap<u32, PathBuf>,
    /// `Player_*.xnb` 路径。
    pub player_sheets: Vec<PathBuf>,
    /// 森林远景。正版 `Background_0.xnb`。
    pub forest_background: Option<PathBuf>,
    /// 生命心。正版 `Heart.xnb`。
    pub hud_heart: Option<PathBuf>,
    /// 魔力星。正版 `Mana.xnb`。
    pub hud_mana: Option<PathBuf>,
    /// 快捷栏 / 背包槽底。正版 `Inventory_Back.xnb`。
    pub hud_inv_back: Option<PathBuf>,
    /// 正版安装根（音效等相对路径）。
    pub install_root: Option<PathBuf>,
}

impl ContentAssets {
    pub fn empty() -> Self {
        Self::default()
    }

    /// 读取 PNG。正版安装没有 PNG。正版贴图走 XNB，不经过这里。
    pub fn load_rgba(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
        let img = PixelImage::load(path).ok()?;
        Some((img.width(), img.height(), img.into_rgba()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_numbered_names_only() {
        let dir = std::env::temp_dir().join(format!("tr-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Item_3.xnb"), b"x").unwrap();
        std::fs::write(dir.join("Item_12.xnb"), b"x").unwrap();
        std::fs::write(dir.join("Tiles_0.xnb"), b"x").unwrap();
        std::fs::write(dir.join("Wall_2.xnb"), b"x").unwrap();
        std::fs::write(dir.join("WallOfFlesh.xnb"), b"x").unwrap();
        std::fs::write(dir.join("NPC_9.xnb"), b"x").unwrap();
        std::fs::write(dir.join("Player_0_0.xnb"), b"x").unwrap();
        std::fs::write(dir.join("Player_1_2.xnb"), b"x").unwrap();
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();

        let index = index_images(&dir).unwrap();
        assert_eq!(index.items.len(), 2);
        assert!(index.tiles.contains_key(&0));
        assert_eq!(index.walls.len(), 1);
        assert!(index.npcs.contains_key(&9));
        assert_eq!(index.players.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boots_install_when_env_set() {
        let Ok(root) = std::env::var("TR_ORIGINAL") else {
            return;
        };
        let assets = boot_content(std::path::Path::new(&root)).expect("boot");
        assert_eq!(assets.boot_frames.len(), 3);
        for tex in &assets.boot_frames {
            assert!(tex.width > 0 && tex.height > 0);
            eprintln!("{} {}x{}", tex.name, tex.width, tex.height);
        }
    }
}
