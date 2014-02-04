//! 内容注册表。方块属性来自 `data/blocks.tbl`，不靠枚举臂。
//!
//! - 运行时数值 ID 仍存世界格里（省空间、存档稳）。
//! - 字符串 `key`（如 `terraria:dirt`）是 mod 稳定标识。
//! - 新增方块只加目录一行。少量 `BlockId` 常量只给仍按名字引用的玩法用。

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use spark_core::{ErrorArg, ErrorArgs};

use crate::weapon::WeaponStats;
use crate::{BlockId, ItemId, WallId};

static CONTENT: OnceLock<ContentRegistry> = OnceLock::new();

/// 内容注册表结构化错误。`Display` 只输出稳定码。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    Sealed,
    DuplicateBlockKey { key: String },
    BlockIdTaken { id: u32 },
    UnknownBlockKey { key: String },
    BlockIdMissing { id: u32 },
    DuplicateItemKey { key: String },
    ItemIdTaken { id: u32 },
    UnknownItemKey { key: String },
    ItemIdMissing { id: u32 },
}

impl ContentError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sealed => "tr.content.sealed",
            Self::DuplicateBlockKey { .. } => "tr.content.duplicate_block_key",
            Self::BlockIdTaken { .. } => "tr.content.block_id_taken",
            Self::UnknownBlockKey { .. } => "tr.content.unknown_block_key",
            Self::BlockIdMissing { .. } => "tr.content.block_id_missing",
            Self::DuplicateItemKey { .. } => "tr.content.duplicate_item_key",
            Self::ItemIdTaken { .. } => "tr.content.item_id_taken",
            Self::UnknownItemKey { .. } => "tr.content.unknown_item_key",
            Self::ItemIdMissing { .. } => "tr.content.item_id_missing",
        }
    }

    /// 供 VM native 映射的短令牌（非用户句子）。
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Sealed => "sealed",
            Self::DuplicateBlockKey { .. } => "duplicate_block_key",
            Self::BlockIdTaken { .. } => "block_id_taken",
            Self::UnknownBlockKey { .. } => "unknown_block_key",
            Self::BlockIdMissing { .. } => "block_id_missing",
            Self::DuplicateItemKey { .. } => "duplicate_item_key",
            Self::ItemIdTaken { .. } => "item_id_taken",
            Self::UnknownItemKey { .. } => "unknown_item_key",
            Self::ItemIdMissing { .. } => "item_id_missing",
        }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Sealed => ErrorArgs::new(),
            Self::DuplicateBlockKey { key } | Self::UnknownBlockKey { key } => {
                ErrorArgs::new().with("key", ErrorArg::String(Arc::from(key.as_str())))
            }
            Self::DuplicateItemKey { key } | Self::UnknownItemKey { key } => {
                ErrorArgs::new().with("key", ErrorArg::String(Arc::from(key.as_str())))
            }
            Self::BlockIdTaken { id }
            | Self::BlockIdMissing { id }
            | Self::ItemIdTaken { id }
            | Self::ItemIdMissing { id } => {
                ErrorArgs::new().with("id", ErrorArg::Unsigned(u64::from(*id)))
            }
        }
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ContentError {}

/// 取得已安装的全局内容表。须先 [`install`]。
pub fn content() -> &'static ContentRegistry {
    CONTENT
        .get()
        .expect("内容表未安装：启动时调用 boot_content / install()")
}

pub fn try_content() -> Option<&'static ContentRegistry> {
    CONTENT.get()
}

/// 安装内容表（进程内一次）。
pub fn install(registry: ContentRegistry) {
    let _ = CONTENT.set(registry);
}

pub fn is_installed() -> bool {
    CONTENT.get().is_some()
}

/// 网格取贴图时的面类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFaceKind {
    Top,
    Bottom,
    Side,
}

/// 方块定义。
#[derive(Debug, Clone)]
pub struct BlockDef {
    pub key: String,
    pub name: String,
    pub solid: bool,
    pub blocks_motion: bool,
    pub ladder: bool,
    pub replaceable: bool,
    pub max_hp: u16,
    pub light_radius: i32,
    pub mine_power_need: Option<u16>,
    pub drop: Option<ItemId>,
    pub color: [f32; 4],
    /// 默认贴图（相对 mod 根，如 `textures/dirt.png`）。
    pub texture: String,
    pub texture_top: String,
    pub texture_side: String,
    pub texture_bottom: String,
    /// 是否自然树干（正版 Trees 一类；树冠走特殊绘制）。
    pub is_tree: bool,
    /// 是否 frame-important（放置时带 `frameX` / `frameY`）。
    pub frame_important: bool,
    /// `Tiles_N` 文件编号。`None` 表示没有这张图。
    pub texture_file: Option<u32>,
    /// 只挡自上而下的脚。
    pub is_platform: bool,
    /// 液体占位（水等）。
    pub is_fluid: bool,
    /// 是否按邻格规则切地形图集（泥土、石头一类）。
    pub framed_terrain: bool,
}

impl BlockDef {
    pub fn emits_light(&self) -> bool {
        self.light_radius > 0
    }

    pub fn mineable(&self) -> bool {
        self.max_hp > 0 && self.max_hp < u16::MAX
    }

    pub fn texture_for(&self, face: BlockFaceKind) -> &str {
        let specific = match face {
            BlockFaceKind::Top => &self.texture_top,
            BlockFaceKind::Side => &self.texture_side,
            BlockFaceKind::Bottom => &self.texture_bottom,
        };
        if specific.is_empty() {
            &self.texture
        } else {
            specific
        }
    }
}

/// 物品定义。
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub key: String,
    pub name: String,
    pub places: Option<BlockId>,
    pub wall: Option<WallId>,
    pub heal: Option<f32>,
    pub mine_power: Option<u16>,
    pub max_durability: u16,
    pub weapon: Option<WeaponStats>,
    pub color: [f32; 3],
    pub in_palette: bool,
    /// 图标贴图路径。
    pub texture: String,
    /// 持有时额外背包格（布袋等）。
    pub bag_bonus_slots: u32,
}

/// 内容注册表。
#[derive(Debug, Clone, Default)]
pub struct ContentRegistry {
    blocks: Vec<Option<BlockDef>>,
    items: Vec<Option<ItemDef>>,
    block_keys: HashMap<String, BlockId>,
    item_keys: HashMap<String, ItemId>,
    palette: Vec<ItemId>,
    next_block: u32,
    next_item: u32,
    sealed: bool,
}

impl ContentRegistry {
    pub fn new() -> Self {
        let mut me = Self::default();
        me.blocks.push(None);
        me.next_block = 1;
        me.next_item = 1;
        me
    }

    pub fn sealed(&self) -> bool {
        self.sealed
    }

    pub fn seal(&mut self) {
        self.palette = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                let def = slot.as_ref()?;
                def.in_palette.then_some(ItemId(i as u32))
            })
            .collect();
        self.sealed = true;
    }

    pub fn palette(&self) -> &[ItemId] {
        &self.palette
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn block_id(&self, key: &str) -> Option<BlockId> {
        self.block_keys.get(key).copied()
    }

    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.item_keys.get(key).copied()
    }

    pub fn block(&self, id: BlockId) -> Option<&BlockDef> {
        self.blocks.get(id.0 as usize).and_then(|s| s.as_ref())
    }

    /// 已登记的方块。顺序按类型 id。
    pub fn iter_blocks(&self) -> impl Iterator<Item = (BlockId, &BlockDef)> {
        self.blocks.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref().map(|def| (BlockId(i as u32), def))
        })
    }

    pub fn item(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.0 as usize).and_then(|s| s.as_ref())
    }

    pub fn block_or_unknown(&self, id: BlockId) -> BlockDef {
        self.block(id).cloned().unwrap_or_else(|| BlockDef {
            key: format!("unknown:{}", id.0),
            name: "未知".into(),
            solid: true,
            blocks_motion: true,
            ladder: false,
            replaceable: false,
            max_hp: 100,
            light_radius: 0,
            mine_power_need: None,
            drop: None,
            color: [1.0, 0.0, 1.0, 1.0],
            texture: String::new(),
            texture_top: String::new(),
            texture_side: String::new(),
            texture_bottom: String::new(),
            is_tree: false,
            frame_important: false,
            texture_file: None,
            is_platform: false,
            is_fluid: false,
            framed_terrain: false,
        })
    }

    pub fn item_or_unknown(&self, id: ItemId) -> ItemDef {
        self.item(id).cloned().unwrap_or_else(|| ItemDef {
            key: format!("unknown:{}", id.0),
            name: "未知".into(),
            places: None,
            wall: None,
            heal: None,
            mine_power: None,
            max_durability: 0,
            weapon: None,
            color: [0.5, 0.5, 0.5],
            in_palette: false,
            texture: String::new(),
            bag_bonus_slots: 0,
        })
    }

    pub fn register_block_at(&mut self, id: BlockId, def: BlockDef) -> Result<(), String> {
        self.ensure_writable()?;
        if self.block_keys.contains_key(&def.key) {
            return Err(format!("方块 key 重复：{}", def.key));
        }
        let idx = id.0 as usize;
        while self.blocks.len() <= idx {
            self.blocks.push(None);
        }
        if self.blocks[idx].is_some() {
            return Err(format!("方块 ID {} 已被占用", id.0));
        }
        self.block_keys.insert(def.key.clone(), id);
        self.blocks[idx] = Some(def);
        self.next_block = self.next_block.max(id.0 + 1);
        Ok(())
    }

    pub fn register_block(&mut self, def: BlockDef) -> Result<BlockId, String> {
        self.ensure_writable()?;
        if self.block_keys.contains_key(&def.key) {
            return Err(format!("方块 key 重复：{}", def.key));
        }
        let id = BlockId(self.next_block);
        self.next_block += 1;
        self.register_block_at(id, def)?;
        Ok(id)
    }

    pub fn set_block_faces(
        &mut self,
        key: &str,
        top: String,
        side: String,
        bottom: String,
    ) -> Result<(), String> {
        self.ensure_writable()?;
        let id = self
            .block_keys
            .get(key)
            .copied()
            .ok_or_else(|| format!("未知方块 key：{key}"))?;
        let slot = self
            .blocks
            .get_mut(id.0 as usize)
            .and_then(|s| s.as_mut())
            .ok_or_else(|| format!("方块 ID {} 未注册", id.0))?;
        slot.texture_top = top;
        slot.texture_side = side;
        slot.texture_bottom = bottom;
        Ok(())
    }

    pub fn register_item_at(&mut self, id: ItemId, def: ItemDef) -> Result<(), String> {
        self.ensure_writable()?;
        if self.item_keys.contains_key(&def.key) {
            return Err(format!("物品 key 重复：{}", def.key));
        }
        let idx = id.0 as usize;
        while self.items.len() <= idx {
            self.items.push(None);
        }
        if self.items[idx].is_some() {
            return Err(format!("物品 ID {} 已被占用", id.0));
        }
        self.item_keys.insert(def.key.clone(), id);
        self.items[idx] = Some(def);
        self.next_item = self.next_item.max(id.0 + 1);
        Ok(())
    }

    pub fn register_item(&mut self, def: ItemDef) -> Result<ItemId, String> {
        self.ensure_writable()?;
        if self.item_keys.contains_key(&def.key) {
            return Err(format!("物品 key 重复：{}", def.key));
        }
        let id = ItemId(self.next_item);
        self.next_item += 1;
        self.register_item_at(id, def)?;
        Ok(id)
    }

    /// 为已注册物品写入武器参数。
    pub fn set_item_weapon(&mut self, key: &str, weapon: WeaponStats) -> Result<(), String> {
        self.ensure_writable()?;
        let id = self
            .item_keys
            .get(key)
            .copied()
            .ok_or_else(|| format!("未知物品 key：{key}"))?;
        let slot = self
            .items
            .get_mut(id.0 as usize)
            .and_then(|s| s.as_mut())
            .ok_or_else(|| format!("物品 ID {} 未注册", id.0))?;
        slot.weapon = Some(weapon);
        Ok(())
    }

    fn ensure_writable(&self) -> Result<(), String> {
        if self.sealed {
            Err("内容表已冻结，无法再注册".into())
        } else {
            Ok(())
        }
    }
}

pub fn block_def(id: BlockId) -> BlockDef {
    content().block_or_unknown(id)
}

pub fn item_def(id: ItemId) -> ItemDef {
    content().item_or_unknown(id)
}

/// 从目录文本登记方块。一行一种，新增方块不必新增 Rust 常量。
///
/// 列（空白分隔，`#` 开头为注释）：
/// `id key name solid motion hp light sheet drop tree framed platform fluid ladder replaceable mine terrain`
/// `sheet` / `drop` / `mine` 用 `-` 表示无。布尔列为 `0` 或 `1`。
pub fn parse_block_catalog(text: &str) -> Result<Vec<(u32, BlockDef)>, String> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.len() != 17 {
            return Err(format!(
                "方块目录第 {} 行列数应为 17，实际 {}",
                lineno + 1,
                t.len()
            ));
        }
        let at = |what: &str| format!("方块目录第 {} 行{what}无效", lineno + 1);
        let id: u32 = t[0].parse().map_err(|_| at(" id "))?;
        if !seen.insert(id) {
            return Err(format!("方块目录重复 id {id}"));
        }
        let key = t[1].to_string();
        let stem = key
            .split(':')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        out.push((
            id,
            BlockDef {
                key,
                name: t[2].to_string(),
                solid: catalog_flag(t[3], lineno)?,
                blocks_motion: catalog_flag(t[4], lineno)?,
                max_hp: t[5].parse().map_err(|_| at(" hp "))?,
                light_radius: t[6].parse().map_err(|_| at(" light "))?,
                texture_file: catalog_opt_u32(t[7], lineno)?,
                drop: catalog_opt_u32(t[8], lineno)?.map(ItemId),
                is_tree: catalog_flag(t[9], lineno)?,
                frame_important: catalog_flag(t[10], lineno)?,
                is_platform: catalog_flag(t[11], lineno)?,
                is_fluid: catalog_flag(t[12], lineno)?,
                ladder: catalog_flag(t[13], lineno)?,
                replaceable: catalog_flag(t[14], lineno)?,
                mine_power_need: catalog_opt_u32(t[15], lineno)?.map(|n| n as u16),
                framed_terrain: catalog_flag(t[16], lineno)?,
                color: [0.5, 0.5, 0.5, 1.0],
                texture: format!("textures/{stem}.png"),
                texture_top: String::new(),
                texture_side: String::new(),
                texture_bottom: String::new(),
            },
        ));
    }
    Ok(out)
}

fn catalog_flag(tok: &str, lineno: usize) -> Result<bool, String> {
    match tok {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(format!("方块目录第 {} 行布尔列须为 0 或 1", lineno + 1)),
    }
}

fn catalog_opt_u32(tok: &str, lineno: usize) -> Result<Option<u32>, String> {
    if tok == "-" {
        return Ok(None);
    }
    tok.parse()
        .map(Some)
        .map_err(|_| format!("方块目录第 {} 行数字无效", lineno + 1))
}

/// 内置方块来自 `data/blocks.tbl`。新增方块只加一行，不要再写常量或 `match`。
pub fn install_builtin_fixture() {
    if is_installed() {
        return;
    }
    let mut reg = ContentRegistry::new();
    let blocks = parse_block_catalog(include_str!("../data/blocks.tbl")).expect("blocks.tbl");
    for (id, def) in blocks {
        let _ = reg.register_block_at(BlockId(id), def);
    }
    let _ = reg.set_block_faces(
        "terraria:grass",
        "textures/grass_top.png".into(),
        "textures/grass_side.png".into(),
        "textures/dirt.png".into(),
    );
    let _ = reg.set_block_faces(
        "terraria:wood",
        "textures/log_top.png".into(),
        "textures/log_side.png".into(),
        "textures/log_top.png".into(),
    );
    reg.seal();
    install(reg);
}

#[cfg(test)]
mod tests {
    use super::parse_block_catalog;
    use crate::ItemId;

    #[test]
    fn catalog_row_does_not_need_a_constant() {
        let text = "400 terraria:example 示例 1 1 10 0 400 6 0 0 0 0 0 0 - 0\n";
        let rows = parse_block_catalog(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 400);
        assert_eq!(rows[0].1.name, "示例");
        assert_eq!(rows[0].1.texture_file, Some(400));
        assert_eq!(rows[0].1.drop, Some(ItemId(6)));
        assert!(!rows[0].1.is_tree);
    }

    #[test]
    fn builtin_catalog_trees_are_id_five() {
        let rows = parse_block_catalog(include_str!("../data/blocks.tbl")).unwrap();
        let tree = rows.iter().find(|row| row.0 == 5).expect("trees");
        assert!(tree.1.is_tree);
        assert!(!tree.1.solid);
        assert!(!tree.1.blocks_motion);
        assert_eq!(tree.1.texture_file, Some(5));
        assert_eq!(tree.1.drop, Some(ItemId(6)));
    }
}
