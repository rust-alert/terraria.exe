//! 房屋空间扫描：封闭房间 + 墙 + 光源 + 家具 的最小可玩判定。
//!
//! 夹具没有椅/桌，用床或工作台充当「舒适家具」。门用平台开口近似。

use tr_core::{BlockId, WallId};

use crate::world::{WORLD_H, WORLD_W, World, x_in_bounds};

/// 室内可通行 / 可计入面积的前景。
fn is_room_fill(id: BlockId) -> bool {
    if id.is_tree() {
        return true;
    }
    matches!(
        id,
        BlockId::AIR
            | BlockId::TORCH
            | BlockId::PLATFORM
            | BlockId::LADDER
            | BlockId::ROPE
            | BlockId::LEAF
            | BlockId::SAPLING
            | BlockId::CHEST
            | BlockId::WORKBENCH
            | BlockId::BED
            | BlockId::FURNACE
            | BlockId::WATER
    )
}

fn is_boundary(id: BlockId) -> bool {
    id.solid() && !id.is_platform()
}

fn is_furniture(id: BlockId) -> bool {
    matches!(id, BlockId::BED | BlockId::WORKBENCH | BlockId::CHEST)
}

fn is_light(id: BlockId) -> bool {
    id.emits_light()
}

/// 房屋判定失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseFail {
    Ok,
    OpenSky,
    TooSmall,
    TooLarge,
    MissingWalls,
    NoLight,
    NoFurniture,
}

impl HouseFail {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "合格房屋",
            Self::OpenSky => "房间未封闭",
            Self::TooSmall => "房间太小",
            Self::TooLarge => "房间太大",
            Self::MissingWalls => "需要铺满背景墙",
            Self::NoLight => "需要火把等光源",
            Self::NoFurniture => "需要床或工作台",
        }
    }
}

/// 一次扫描结果。
#[derive(Debug, Clone)]
pub struct HouseReport {
    pub fail: HouseFail,
    /// 室内格。
    pub tiles: Vec<(i32, i32)>,
    /// 建议站立点（最低实心地板上方）。
    pub stand: Option<(i32, i32)>,
}

impl HouseReport {
    pub fn valid(&self) -> bool {
        self.fail == HouseFail::Ok && self.stand.is_some()
    }
}

/// 从种子格洪水填充封闭空间并评分。
pub fn scan_from(world: &World, seed_tx: i32, seed_ty: i32) -> HouseReport {
    let mut empty = HouseReport {
        fail: HouseFail::OpenSky,
        tiles: Vec::new(),
        stand: None,
    };
    if !x_in_bounds(seed_tx) || seed_ty < 0 || seed_ty >= WORLD_H {
        return empty;
    }
    if !is_room_fill(world.get(seed_tx, seed_ty)) {
        // 种子在实心上：向下找一格空气再扫。
        if seed_ty + 1 < WORLD_H && is_room_fill(world.get(seed_tx, seed_ty + 1)) {
            return scan_from(world, seed_tx, seed_ty + 1);
        }
        if seed_ty > 0 && is_room_fill(world.get(seed_tx, seed_ty - 1)) {
            return scan_from(world, seed_tx, seed_ty - 1);
        }
        return empty;
    }

    const MAX_TILES: usize = 400;
    const MIN_TILES: usize = 30;

    let mut visited = vec![false; (WORLD_W * WORLD_H) as usize];
    let mut stack = vec![(seed_tx, seed_ty)];
    let mut tiles = Vec::new();
    let mut open = false;

    while let Some((x, y)) = stack.pop() {
        if !x_in_bounds(x) || y < 0 || y >= WORLD_H {
            open = true;
            continue;
        }
        let idx = (y * WORLD_W + x) as usize;
        if visited[idx] {
            continue;
        }
        let id = world.get(x, y);
        if is_boundary(id) {
            continue;
        }
        if !is_room_fill(id) {
            // 非边界也非室内 → 当作开口。
            open = true;
            continue;
        }
        visited[idx] = true;
        tiles.push((x, y));
        if tiles.len() > MAX_TILES {
            return HouseReport {
                fail: HouseFail::TooLarge,
                tiles,
                stand: None,
            };
        }
        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }

    if open {
        empty.tiles = tiles;
        empty.fail = HouseFail::OpenSky;
        return empty;
    }
    if tiles.len() < MIN_TILES {
        return HouseReport {
            fail: HouseFail::TooSmall,
            tiles,
            stand: None,
        };
    }

    let mut wall_ok = 0usize;
    let mut has_light = false;
    let mut has_furn = false;
    for &(x, y) in &tiles {
        if world.get_wall(x, y) != WallId::NONE {
            wall_ok += 1;
        }
        let id = world.get(x, y);
        if is_light(id) {
            has_light = true;
        }
        if is_furniture(id) {
            has_furn = true;
        }
    }
    // 至少 80% 室内格有墙。
    if wall_ok * 5 < tiles.len() * 4 {
        return HouseReport {
            fail: HouseFail::MissingWalls,
            tiles,
            stand: None,
        };
    }
    if !has_light {
        return HouseReport {
            fail: HouseFail::NoLight,
            tiles,
            stand: None,
        };
    }
    if !has_furn {
        return HouseReport {
            fail: HouseFail::NoFurniture,
            tiles,
            stand: None,
        };
    }

    let stand = pick_stand(world, &tiles);
    HouseReport {
        fail: HouseFail::Ok,
        tiles,
        stand,
    }
}

fn pick_stand(world: &World, tiles: &[(i32, i32)]) -> Option<(i32, i32)> {
    // 选室内最低、脚下有实心的格。
    let mut best: Option<(i32, i32, i32)> = None; // (score_y, x, y)
    for &(x, y) in tiles {
        if !is_room_fill(world.get(x, y)) {
            continue;
        }
        let below = world.get(x, y + 1);
        if !below.solid() {
            continue;
        }
        let score = y;
        match best {
            None => best = Some((score, x, y)),
            Some((sy, _, _)) if score >= sy => best = Some((score, x, y)),
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

/// 已登记的房屋槽（可入住）。
#[derive(Debug, Clone)]
pub struct HouseSlot {
    pub stand: (i32, i32),
    pub tiles: Vec<(i32, i32)>,
    pub occupied: Option<crate::npc::TownKind>,
}

impl HouseSlot {
    pub fn from_report(report: &HouseReport) -> Option<Self> {
        if !report.valid() {
            return None;
        }
        Some(Self {
            stand: report.stand?,
            tiles: report.tiles.clone(),
            occupied: None,
        })
    }

    pub fn contains_tile(&self, tx: i32, ty: i32) -> bool {
        self.tiles.iter().any(|&(x, y)| x == tx && y == ty)
    }

    /// 与另一套房是否显著重叠（避免重复登记）。
    pub fn overlaps(&self, other: &HouseReport) -> bool {
        let Some(os) = other.stand else {
            return false;
        };
        if self.stand == os {
            return true;
        }
        let shared = self
            .tiles
            .iter()
            .filter(|t| other.tiles.iter().any(|o| o == *t))
            .count();
        shared * 2 > self.tiles.len().min(other.tiles.len())
    }
}

/// 在玩家附近查询房屋资格（只读，不登记）。返回提示与高亮格。
pub fn query_near(world: &World, px: f32, py: f32) -> (String, Vec<(i32, i32)>) {
    let tx = (px / crate::world::TILE).floor() as i32;
    let ty = (py / crate::world::TILE).floor() as i32;
    let report = scan_from(world, tx, ty);
    let msg = if report.valid() {
        if let Some((sx, sy)) = report.stand {
            format!("房屋：合格（站立格 {sx},{sy}）· 入住改由住房分配，不再按 H 登记")
        } else {
            "房屋：合格".into()
        }
    } else {
        format!("房屋：{}", report.fail.label())
    };
    (msg, report.tiles)
}
