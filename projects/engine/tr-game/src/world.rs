//! 侧视地表世界：确定性高度 + 方块表 + 地面掉落物。
//!
//! 坐标：`y = 0` 为世界顶（天空），`y` 增大向地心。
//! **X 为左右回环圆柱**（行星拓扑）：出左缘进右缘。

use std::collections::HashMap;
use std::f32::consts::TAU;
use tr_core::{BiomeId, BlockId, FluidLevel, ItemId, WallId, biome_at};

pub const WORLD_W: i32 = 160;
pub const WORLD_H: i32 = 72;
pub const TILE: f32 = 20.0;

/// 世界宽度（世界单位）。
pub fn world_pixel_w() -> f32 {
    WORLD_W as f32 * TILE
}

/// 格坐标 X 回环到 `[0, WORLD_W)`。
pub fn wrap_tx(x: i32) -> i32 {
    let w = WORLD_W;
    ((x % w) + w) % w
}

/// 世界坐标 X 回环到 `[0, world_pixel_w)`。
pub fn wrap_xf(x: f32) -> f32 {
    x.rem_euclid(world_pixel_w())
}

/// 从 `from` 到 `to` 的最短有符号水平距离（可跨缝）。
pub fn wrap_delta_x(from: f32, to: f32) -> f32 {
    let w = world_pixel_w();
    let mut d = to - from;
    if d > w * 0.5 {
        d -= w;
    } else if d < -w * 0.5 {
        d += w;
    }
    d
}

/// 将方块左缘 `tx*TILE` 挪到最靠近 `near_x` 的周期像。
pub fn tile_x_near(tx: i32, near_x: f32) -> f32 {
    let bx = wrap_tx(tx) as f32 * TILE;
    near_x + wrap_delta_x(near_x, bx)
}

/// 与 `player::HIT_H` 同式，避免循环依赖。
const PLAYER_HIT_H: f32 = TILE * (42.0 / 16.0);
const PLAYER_HIT_W: f32 = TILE * (20.0 / 16.0);

/// 地面掉落物（打碎后落地，走近才进背包）。
#[derive(Debug, Clone)]
pub struct GroundDrop {
    pub x: f32,
    pub y: f32,
    pub item: ItemId,
    pub count: u32,
    pub bob: f32,
}

#[derive(Debug, Clone)]
pub struct World {
    pub seed: u64,
    blocks: Vec<BlockId>,
    walls: Vec<WallId>,
    /// 与方块表同尺寸；仅 `WATER` 格有效，其它忽略。
    fluid: Vec<FluidLevel>,
    /// 稀疏：仅记录**受伤未碎**格的剩余 HP。满血或不存在 = 未受伤。
    damage_hp: HashMap<(i32, i32), u16>,
    /// 背景墙受伤剩余 HP。
    wall_hp: HashMap<(i32, i32), u16>,
    /// 树苗生长倒计时（秒）；键为格坐标。
    grow_t: HashMap<(i32, i32), f32>,
    /// 木箱内容（稀疏）。
    pub chests: HashMap<(i32, i32), HashMap<ItemId, u32>>,
    pub drops: Vec<GroundDrop>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        let n = (WORLD_W * WORLD_H) as usize;
        let mut blocks = vec![BlockId::AIR; n];
        let mut walls = vec![WallId::NONE; n];
        let mut w = Self {
            seed,
            blocks: Vec::new(),
            walls: Vec::new(),
            fluid: vec![FluidLevel::SOURCE; n],
            damage_hp: HashMap::new(),
            wall_hp: HashMap::new(),
            grow_t: HashMap::new(),
            chests: HashMap::new(),
            drops: Vec::new(),
        };
        for x in 0..WORLD_W {
            let h = surface_height(seed, x);
            let biome = biome_at(seed, x);
            for y in 0..WORLD_H {
                let idx = (y * WORLD_W + x) as usize;
                let id = if y < h {
                    BlockId::AIR
                } else if y == h {
                    match biome {
                        BiomeId::Desert => BlockId::SAND,
                        BiomeId::Tundra => BlockId::SNOW,
                        BiomeId::Meadow | BiomeId::Forest => BlockId::GRASS,
                    }
                } else if y < h + 5 {
                    match biome {
                        BiomeId::Desert => {
                            if y < h + 3 {
                                BlockId::SAND
                            } else {
                                BlockId::DIRT
                            }
                        }
                        BiomeId::Tundra => {
                            if y < h + 2 {
                                BlockId::SNOW
                            } else {
                                BlockId::DIRT
                            }
                        }
                        _ => BlockId::DIRT,
                    }
                } else {
                    // 深层默认石头。铜走洞穴旁矿脉；铁与废料只在更深处稀疏出现。
                    let depth = y - h;
                    let roll = hash2(seed, x, y);
                    if depth >= 14 && roll % 67 == 0 {
                        BlockId::IRON_ORE
                    } else if depth >= 10 && roll % 53 == 0 {
                        BlockId::SCRAP
                    } else {
                        BlockId::STONE
                    }
                };
                blocks[idx] = id;
                // 地表以下铺背景墙（挖穿后仍见岩壁）
                if y > h {
                    walls[idx] = if y < h + 5 {
                        WallId::DIRT
                    } else {
                        WallId::STONE
                    };
                } else if y == h {
                    walls[idx] = WallId::DIRT;
                }
            }
        }
        w.blocks = blocks;
        w.walls = walls;

        // 逃生舱：宽 4、净空 ≥3 格
        let spawn_x = 24;
        let sh = surface_height(seed, spawn_x);
        for dx in 0..4 {
            let x = wrap_tx(spawn_x + dx);
            if w.y_in_bounds(sh) {
                w.set(x, sh, BlockId::POD);
            }
        }
        for dy in 1..=4 {
            for dx in 0..4 {
                let x = wrap_tx(spawn_x + dx);
                let y = sh - dy;
                if !w.y_in_bounds(y) {
                    continue;
                }
                let wall = dx == 0 || dx == 3 || dy == 4;
                let hatch = dy == 4 && (dx == 1 || dx == 2);
                if hatch {
                    w.set(x, y, BlockId::AIR);
                } else if wall {
                    w.set(x, y, BlockId::POD);
                } else {
                    w.set(x, y, BlockId::AIR);
                }
            }
        }

        w.plant_trees();
        w.carve_and_fill_lakes();
        w.bake_fluid_flow();
        w.carve_shallow_caves();
        w.place_copper_veins();

        // 裂痕锚放在环行远处，不挡出生后的第一段地表。
        let wx = wrap_tx(spawn_x + 108);
        let wh = surface_height(seed, wx);
        if w.get(wx, wh - 1) == BlockId::AIR || w.get(wx, wh - 1) == BlockId::LEAF {
            w.set(wx, wh - 1, BlockId::WARP);
        } else {
            w.set(wx, wh - 2, BlockId::WARP);
        }
        // 逃生舱旁一枚火把，夜里好辨认
        let (sx, _) = w.spawn_pos();
        let stx = wrap_tx((sx / TILE).floor() as i32 + 2);
        let sty = surface_height(seed, stx) - 2;
        if w.get(stx, sty) == BlockId::AIR {
            w.set(stx, sty, BlockId::TORCH);
        }
        // 舱旁补给木箱
        let ctx = wrap_tx(stx + 2);
        let cty = surface_height(seed, ctx) - 1;
        if w.get(ctx, cty) == BlockId::AIR {
            w.set(ctx, cty, BlockId::CHEST);
            if let Some(chest) = w.chest_at(ctx, cty) {
                chest.insert(ItemId::GEL, 3);
                chest.insert(ItemId::WOOD, 6);
                chest.insert(ItemId::TORCH, 2);
                chest.insert(ItemId::LADDER, 8);
            }
        }

        w
    }

    /// 地表种树：树干 `WOOD`，树冠 `LEAF`（不挡碰撞）。
    fn plant_trees(&mut self) {
        let mut x = 8;
        while x < WORLD_W - 4 {
            let wx = wrap_tx(x);
            let near_pod = (0..6).any(|d| {
                let t = wrap_tx(24 + d);
                let dist = (wx - t)
                    .rem_euclid(WORLD_W)
                    .min((t - wx).rem_euclid(WORLD_W));
                dist < 8
            });
            let h = surface_height(self.seed, wx);
            let biome = biome_at(self.seed, wx);
            let plantable = matches!(
                self.get(wx, h),
                BlockId::GRASS | BlockId::SNOW | BlockId::DIRT
            );
            let roll = hash2(self.seed, wx, 91) % 100;
            if !near_pod && plantable && roll < biome.tree_chance() as u64 {
                let trunk = 4 + (hash2(self.seed, wx, 7) % 4) as i32;
                self.grow_tree_at(wx, h, trunk);
                x += 5 + (hash2(self.seed, wx, 13) % 4) as i32;
            } else {
                x += 2;
            }
        }
    }

    /// 在地表挖浅湖并灌满源水（避开出生舱）。
    fn carve_and_fill_lakes(&mut self) {
        let centers = [wrap_tx(24 + 38), wrap_tx(24 + 95), wrap_tx(24 + 128)];
        for (i, &cx) in centers.iter().enumerate() {
            let half_w = 6 + (hash2(self.seed, cx, 401 + i as i32) % 4) as i32;
            let depth = 3 + (hash2(self.seed, cx, 503 + i as i32) % 3) as i32;
            let sh = surface_height(self.seed, cx);
            let water_y = sh; // 水面约在原地表
            // 挖盆：向下加深，两侧抬高岸线
            for dx in -half_w..=half_w {
                let x = wrap_tx(cx + dx);
                let t = dx.abs() as f32 / half_w as f32;
                let dig = ((1.0 - t * t) * depth as f32).round() as i32;
                if dig <= 0 {
                    continue;
                }
                let basin_bottom = water_y + dig;
                for y in (water_y - 1)..=basin_bottom {
                    if !self.y_in_bounds(y) {
                        continue;
                    }
                    let id = self.get(x, y);
                    if id == BlockId::POD || id == BlockId::WARP || id == BlockId::CHEST {
                        continue;
                    }
                    if y < water_y {
                        // 湖面上空清障
                        if !id.solid()
                            || matches!(id, BlockId::LEAF | BlockId::SAPLING | BlockId::WOOD)
                        {
                            self.set(x, y, BlockId::AIR);
                        }
                    } else {
                        self.set(x, y, BlockId::AIR);
                    }
                }
                // 湖底铺沙/泥
                if self.y_in_bounds(basin_bottom) {
                    let bed = match biome_at(self.seed, x) {
                        BiomeId::Desert => BlockId::SAND,
                        BiomeId::Tundra => BlockId::STONE,
                        _ => BlockId::DIRT,
                    };
                    self.set(x, basin_bottom, bed);
                    if self.get_wall(x, basin_bottom) == WallId::NONE {
                        self.set_wall(x, basin_bottom, WallId::DIRT);
                    }
                }
                // 灌源水：水面以下、湖底以上
                for y in water_y..basin_bottom {
                    if !self.y_in_bounds(y) {
                        continue;
                    }
                    if self.get(x, y) == BlockId::AIR {
                        self.set_water(x, y, FluidLevel::SOURCE);
                    }
                }
            }
            // 岸边一侧挖浅溢流槽，方便烘焙出流动水位
            let spill_dir = if hash2(self.seed, cx, 607) % 2 == 0 {
                1
            } else {
                -1
            };
            let sx = wrap_tx(cx + spill_dir * (half_w + 1));
            let sy = surface_height(self.seed, sx);
            for step in 0..5 {
                let x = wrap_tx(sx + spill_dir * step);
                let y = sy + step.min(2);
                if !self.y_in_bounds(y) {
                    break;
                }
                let id = self.get(x, y);
                if id.solid() && id != BlockId::POD {
                    self.set(x, y, BlockId::AIR);
                }
                let below = y + 1;
                if self.y_in_bounds(below) && self.get(x, below) == BlockId::AIR {
                    // 留空给下落水
                } else if self.y_in_bounds(below) && !self.get(x, below).solid() {
                    // keep
                }
            }
        }
    }

    /// 一次性按 MC 规则烘焙流动/下落水位。
    fn bake_fluid_flow(&mut self) {
        for _ in 0..48 {
            if !self.tick_fluids_once() {
                break;
            }
        }
    }

    /// 浅层洞穴：水平虫洞，避开逃生舱正下方。
    fn carve_shallow_caves(&mut self) {
        const SPAWN_X: i32 = 24;
        for i in 0..12i32 {
            let mut x = wrap_tx((hash2(self.seed, i, 910) % WORLD_W as u64) as i32);
            let dist = (x - SPAWN_X)
                .rem_euclid(WORLD_W)
                .min((SPAWN_X - x).rem_euclid(WORLD_W));
            if dist < 18 {
                continue;
            }
            let sh = surface_height(self.seed, x);
            let mut y = (sh + 7 + (hash2(self.seed, i, 911) % 5) as i32).min(WORLD_H - 4);
            let steps = 16 + (hash2(self.seed, i, 912) % 12) as i32;
            let mut dir = if hash2(self.seed, i, 913) % 2 == 0 {
                1
            } else {
                -1
            };
            for s in 0..steps {
                self.carve_cave_cell(x, y);
                self.carve_cave_cell(x, (y + 1).min(WORLD_H - 2));
                let roll = hash2(self.seed, x, y + s);
                if roll % 5 == 0 {
                    y += 1;
                } else if roll % 5 == 1 {
                    y -= 1;
                }
                if roll % 9 == 0 {
                    dir = -dir;
                }
                let sh_here = surface_height(self.seed, x);
                y = y.clamp(sh_here + 6, (sh_here + 20).min(WORLD_H - 3));
                x = wrap_tx(x + dir);
            }
        }
    }

    fn carve_cave_cell(&mut self, x: i32, y: i32) {
        if !self.y_in_bounds(y) {
            return;
        }
        let id = self.get(x, y);
        if !matches!(
            id,
            BlockId::STONE | BlockId::DIRT | BlockId::SCRAP | BlockId::IRON_ORE | BlockId::SAND
        ) {
            return;
        }
        self.set(x, y, BlockId::AIR);
        if self.get_wall(x, y) == WallId::NONE {
            self.set_wall(x, y, WallId::STONE);
        }
    }

    /// 在洞穴壁旁铺一小簇铜矿，而不是全图散点。
    fn place_copper_veins(&mut self) {
        let mut placed = 0u32;
        for y in 10..WORLD_H - 2 {
            for x in 0..WORLD_W {
                if placed >= 16 {
                    return;
                }
                if self.get(x, y) != BlockId::AIR {
                    continue;
                }
                let sh = surface_height(self.seed, x);
                let depth = y - sh;
                if !(6..=22).contains(&depth) {
                    continue;
                }
                if hash2(self.seed, x, y) % 19 != 0 {
                    continue;
                }
                let neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                for (dx, dy) in neighbors {
                    let nx = wrap_tx(x + dx);
                    let ny = y + dy;
                    if self.get(nx, ny) != BlockId::STONE {
                        continue;
                    }
                    self.paint_copper_blob(nx, ny);
                    placed += 1;
                    break;
                }
            }
        }
    }

    fn paint_copper_blob(&mut self, cx: i32, cy: i32) {
        for dy in -1..=1 {
            for dx in -2..=2 {
                if dx == 0 && dy == 0 {
                    self.set(cx, cy, BlockId::COPPER_ORE);
                    continue;
                }
                if hash2(self.seed, cx + dx, cy + dy) % 3 == 0 {
                    continue;
                }
                let x = wrap_tx(cx + dx);
                let y = cy + dy;
                if self.get(x, y) == BlockId::STONE {
                    self.set(x, y, BlockId::COPPER_ORE);
                }
            }
        }
    }

    /// 每帧调用：推进若干轮流体扩散 / 退水。
    pub fn tick_fluids(&mut self, rounds: u32) {
        for _ in 0..rounds.max(1) {
            if !self.tick_fluids_once() {
                break;
            }
        }
    }

    /// 单轮流体步进。有变化返回 `true`。
    fn tick_fluids_once(&mut self) -> bool {
        let mut sources = Vec::new();
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                if self.get(x, y) == BlockId::WATER {
                    sources.push((x, y, self.fluid_level(x, y)));
                }
            }
        }
        let mut changed = false;

        // 扩散：下落优先，再水平。
        for (x, y, lv) in &sources {
            let x = *x;
            let y = *y;
            let lv = *lv;
            let below = y + 1;
            if self.y_in_bounds(below) && self.get(x, below) == BlockId::AIR {
                self.set_water(x, below, FluidLevel::FALLING);
                changed = true;
                continue;
            }
            let spread_from = if lv.is_falling() { FluidLevel(1) } else { lv };
            let Some(next) = spread_from.spread_next() else {
                continue;
            };
            for dx in [-1i32, 1] {
                let nx = wrap_tx(x + dx);
                let cur = self.get(nx, y);
                if cur == BlockId::AIR {
                    self.set_water(nx, y, next);
                    changed = true;
                } else if cur == BlockId::WATER {
                    let old = self.fluid_level(nx, y);
                    if !old.is_source() && !old.is_falling() && next.0 < old.0 {
                        self.set_water(nx, y, next);
                        changed = true;
                    }
                }
            }
        }

        // 退水：非源格若失去支撑则变浅或消失。
        let mut shrink = Vec::new();
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                if self.get(x, y) != BlockId::WATER {
                    continue;
                }
                let lv = self.fluid_level(x, y);
                if lv.is_source() {
                    continue;
                }
                if !self.fluid_supported(x, y, lv) {
                    shrink.push((x, y, lv));
                }
            }
        }
        for (x, y, lv) in shrink {
            match lv.recede() {
                Some(n) => {
                    self.set_water(x, y, n);
                    changed = true;
                }
                None => {
                    self.clear_water(x, y);
                    changed = true;
                }
            }
        }
        changed
    }

    fn fluid_supported(&self, x: i32, y: i32, lv: FluidLevel) -> bool {
        if lv.is_falling() {
            let above = y - 1;
            return self.y_in_bounds(above) && self.get(x, above) == BlockId::WATER;
        }
        // 水平流：需要邻格提供更「满」的水，或上方下落/源。
        let above = y - 1;
        if self.y_in_bounds(above) && self.get(x, above) == BlockId::WATER {
            let al = self.fluid_level(x, above);
            if al.is_source() || al.is_falling() || al.0 < lv.0 {
                return true;
            }
        }
        for dx in [-1i32, 1] {
            let nx = wrap_tx(x + dx);
            if self.get(nx, y) != BlockId::WATER {
                continue;
            }
            let nl = self.fluid_level(nx, y);
            if nl.is_source() || nl.is_falling() {
                return true;
            }
            if !nl.is_falling() && nl.0 < lv.0 {
                return true;
            }
        }
        false
    }

    pub fn clear_water(&mut self, x: i32, y: i32) {
        if !self.y_in_bounds(y) {
            return;
        }
        let x = wrap_tx(x);
        let idx = (y * WORLD_W + x) as usize;
        if self.blocks[idx] == BlockId::WATER {
            self.blocks[idx] = BlockId::AIR;
            self.fluid[idx] = FluidLevel::SOURCE;
        }
    }

    /// 矩形与水体重叠比例（0..=1），按水位高度估算。
    pub fn water_overlap_ratio(&self, px: f32, py: f32, pw: f32, ph: f32) -> f32 {
        let min_tx = (px / TILE).floor() as i32;
        let max_tx = ((px + pw) / TILE).floor() as i32;
        let min_ty = (py / TILE).floor() as i32;
        let max_ty = ((py + ph) / TILE).floor() as i32;
        let body = (pw * ph).max(1.0);
        let mut water_area = 0.0f32;
        for ty in min_ty..=max_ty {
            if !self.y_in_bounds(ty) {
                continue;
            }
            for tx in min_tx..=max_tx {
                if self.get(tx, ty) != BlockId::WATER {
                    continue;
                }
                let fill = self.fluid_level(tx, ty).fill_ratio();
                let bx = tile_x_near(tx, px + pw * 0.5);
                let by = ty as f32 * TILE;
                let water_top = by + TILE * (1.0 - fill);
                let ix0 = px.max(bx);
                let iy0 = py.max(water_top);
                let ix1 = (px + pw).min(bx + TILE);
                let iy1 = (py + ph).min(by + TILE);
                if ix1 > ix0 && iy1 > iy0 {
                    water_area += (ix1 - ix0) * (iy1 - iy0);
                }
            }
        }
        (water_area / body).clamp(0.0, 1.0)
    }

    fn fluid_idx(x: i32, y: i32) -> usize {
        (y * WORLD_W + wrap_tx(x)) as usize
    }

    pub fn fluid_level(&self, x: i32, y: i32) -> FluidLevel {
        if !self.y_in_bounds(y) || self.get(x, y) != BlockId::WATER {
            return FluidLevel::SOURCE;
        }
        self.fluid[Self::fluid_idx(x, y)].clamp_valid()
    }

    pub fn set_water(&mut self, x: i32, y: i32, level: FluidLevel) {
        if !self.y_in_bounds(y) {
            return;
        }
        let x = wrap_tx(x);
        let idx = (y * WORLD_W + x) as usize;
        self.blocks[idx] = BlockId::WATER;
        self.fluid[idx] = level.clamp_valid();
        self.damage_hp.remove(&(x, y));
        self.grow_t.remove(&(x, y));
    }

    /// 在地表草皮上长出一棵树（`surface_y` 为草皮格 Y）。
    pub fn grow_tree_at(&mut self, tx: i32, surface_y: i32, trunk: i32) {
        let tx = wrap_tx(tx);
        let trunk = trunk.clamp(3, 8);
        // 清掉树苗
        self.grow_t.remove(&(tx, surface_y - 1));
        for i in 1..=trunk {
            let y = surface_y - i;
            if !self.y_in_bounds(y) {
                continue;
            }
            let cur = self.get(tx, y);
            if matches!(cur, BlockId::AIR | BlockId::LEAF | BlockId::SAPLING) {
                self.set(tx, y, BlockId::WOOD);
            }
        }
        let top = surface_y - trunk;
        for dy in -2i32..=1 {
            for dx in -2i32..=2 {
                if dx.abs() + dy.abs() > 3 {
                    continue;
                }
                let lx = wrap_tx(tx + dx);
                let ly = top + dy;
                if !self.y_in_bounds(ly) {
                    continue;
                }
                let cur = self.get(lx, ly);
                if matches!(cur, BlockId::AIR | BlockId::SAPLING) {
                    self.set(lx, ly, BlockId::LEAF);
                }
            }
        }
    }

    /// 树苗是否可种在 `(tx,ty)`：格为空/可覆盖，下方为草皮，上方净空。
    pub fn can_plant_sapling(&self, tx: i32, ty: i32) -> bool {
        if !self.y_in_bounds(ty) {
            return false;
        }
        let cur = self.get(tx, ty);
        if !matches!(cur, BlockId::AIR | BlockId::LEAF | BlockId::SAPLING) {
            return false;
        }
        let soil = self.get(tx, ty + 1);
        if !matches!(
            soil,
            BlockId::GRASS | BlockId::DIRT | BlockId::SNOW | BlockId::SAND
        ) {
            return false;
        }
        // 上方至少 4 格可长
        for i in 1..=4 {
            let above = self.get(tx, ty - i);
            if above.blocks_motion() {
                return false;
            }
        }
        true
    }

    /// 放置树苗并启动生长计时。
    pub fn plant_sapling(&mut self, tx: i32, ty: i32) -> bool {
        if !self.can_plant_sapling(tx, ty) {
            return false;
        }
        let tx = wrap_tx(tx);
        self.set(tx, ty, BlockId::SAPLING);
        true
    }

    /// 推进树苗生长与草皮蔓延。
    pub fn tick_growth(&mut self, dt: f32) {
        // 树苗
        let keys: Vec<(i32, i32)> = self.grow_t.keys().copied().collect();
        let mut matured = Vec::new();
        for key in keys {
            let Some(left) = self.grow_t.get_mut(&key) else {
                continue;
            };
            *left -= dt;
            if *left <= 0.0 {
                matured.push(key);
            }
        }
        for (tx, ty) in matured {
            self.grow_t.remove(&(tx, ty));
            if self.get(tx, ty) != BlockId::SAPLING {
                continue;
            }
            // 土壤在树苗下方
            let soil_y = ty + 1;
            let soil = self.get(tx, soil_y);
            if soil != BlockId::GRASS && soil != BlockId::DIRT {
                // 失去土壤 → 枯萎成空气
                self.set(tx, ty, BlockId::AIR);
                continue;
            }
            if soil == BlockId::DIRT {
                self.set(tx, soil_y, BlockId::GRASS);
            }
            let trunk = 4 + (hash2(self.seed, tx, ty.wrapping_add(3)) % 4) as i32;
            self.grow_tree_at(tx, soil_y, trunk);
        }

        // 草皮缓慢蔓延到邻接泥土（每帧抽样，避免全图扫描）
        let sample = ((self.seed ^ (dt.to_bits() as u64)) % WORLD_W as u64) as i32;
        for k in 0..6 {
            let x = wrap_tx(sample + k * 17);
            let h = surface_height(self.seed, x);
            if self.get(x, h) != BlockId::DIRT {
                continue;
            }
            let neighbor_grass = [-1i32, 1]
                .iter()
                .any(|dx| self.get(x + dx, h) == BlockId::GRASS)
                || self.get(x, h - 1) == BlockId::GRASS;
            if neighbor_grass && hash2(self.seed, x, h.wrapping_add(41)) % 40 == 0 {
                // 上方须为空气才铺草
                if self.get(x, h - 1) == BlockId::AIR || !self.get(x, h - 1).blocks_motion() {
                    self.set(x, h, BlockId::GRASS);
                }
            }
        }
    }

    pub fn y_in_bounds(&self, y: i32) -> bool {
        y >= 0 && y < WORLD_H
    }

    /// Y 合法即可；X 始终回环，无「出界」。
    pub fn in_bounds(&self, _x: i32, y: i32) -> bool {
        self.y_in_bounds(y)
    }

    pub fn get(&self, x: i32, y: i32) -> BlockId {
        if !self.y_in_bounds(y) {
            return BlockId::STONE;
        }
        let x = wrap_tx(x);
        self.blocks[(y * WORLD_W + x) as usize]
    }

    pub fn get_wall(&self, x: i32, y: i32) -> WallId {
        if !self.y_in_bounds(y) {
            return WallId::NONE;
        }
        let x = wrap_tx(x);
        self.walls[(y * WORLD_W + x) as usize]
    }

    pub fn set_wall(&mut self, x: i32, y: i32, id: WallId) {
        if !self.y_in_bounds(y) {
            return;
        }
        let x = wrap_tx(x);
        self.walls[(y * WORLD_W + x) as usize] = id;
        self.wall_hp.remove(&(x, y));
    }

    /// 墙剩余 HP。
    pub fn wall_hp_left(&self, x: i32, y: i32) -> u16 {
        let id = self.get_wall(x, y);
        let max = id.max_hp();
        if max == 0 {
            return 0;
        }
        let x = wrap_tx(x);
        self.wall_hp.get(&(x, y)).copied().unwrap_or(max)
    }

    pub fn wall_damage_ratio(&self, x: i32, y: i32) -> f32 {
        let id = self.get_wall(x, y);
        let max = id.max_hp();
        if max == 0 {
            return 0.0;
        }
        let left = self.wall_hp_left(x, y);
        1.0 - (left as f32 / max as f32)
    }

    /// 对背景墙造成伤害；打碎则清空并返回原墙。
    pub fn apply_wall_damage(&mut self, x: i32, y: i32, dmg: u16) -> Option<WallId> {
        if !self.y_in_bounds(y) || dmg == 0 {
            return None;
        }
        let id = self.get_wall(x, y);
        if !id.mineable() {
            return None;
        }
        let max = id.max_hp();
        let x = wrap_tx(x);
        let key = (x, y);
        let left = self.wall_hp.get(&key).copied().unwrap_or(max);
        let next = left.saturating_sub(dmg);
        if next == 0 {
            self.wall_hp.remove(&key);
            self.walls[(y * WORLD_W + x) as usize] = WallId::NONE;
            Some(id)
        } else {
            self.wall_hp.insert(key, next);
            None
        }
    }

    pub fn set(&mut self, x: i32, y: i32, id: BlockId) {
        if !self.y_in_bounds(y) {
            return;
        }
        let x = wrap_tx(x);
        let prev = self.blocks[(y * WORLD_W + x) as usize];
        self.blocks[(y * WORLD_W + x) as usize] = id;
        self.damage_hp.remove(&(x, y));
        if id == BlockId::WATER {
            // 外部裸 set 默认为源；精细水位走 set_water
            if prev != BlockId::WATER {
                self.fluid[(y * WORLD_W + x) as usize] = FluidLevel::SOURCE;
            }
        } else {
            self.fluid[(y * WORLD_W + x) as usize] = FluidLevel::SOURCE;
        }
        if prev == BlockId::SAPLING && id != BlockId::SAPLING {
            self.grow_t.remove(&(x, y));
        }
        if prev == BlockId::CHEST && id != BlockId::CHEST {
            if let Some(inv) = self.chests.remove(&(x, y)) {
                for (item, n) in inv {
                    if n > 0 {
                        self.spawn_drop_at_tile(x, y, item, n);
                    }
                }
            }
        }
        if id == BlockId::SAPLING && prev != BlockId::SAPLING {
            let secs = 10.0 + (hash2(self.seed, x, y) % 90) as f32 * 0.1;
            self.grow_t.insert((x, y), secs);
        }
        if id == BlockId::CHEST && prev != BlockId::CHEST {
            self.chests.entry((x, y)).or_default();
        }
    }

    pub fn chest_at(&mut self, x: i32, y: i32) -> Option<&mut HashMap<ItemId, u32>> {
        let x = wrap_tx(x);
        if self.get(x, y) != BlockId::CHEST {
            return None;
        }
        Some(self.chests.entry((x, y)).or_default())
    }

    pub fn near_chest(&self, px: f32, py: f32, pw: f32, ph: f32) -> Option<(i32, i32)> {
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let tx = wrap_tx((cx / TILE).floor() as i32);
        let ty = (cy / TILE).floor() as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let x = wrap_tx(tx + dx);
                let y = ty + dy;
                if self.get(x, y) == BlockId::CHEST {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// 导出木箱（存档）。
    pub fn encode_chests(&self) -> String {
        self.chests
            .iter()
            .map(|((x, y), inv)| {
                let body = inv
                    .iter()
                    .filter(|(_, n)| **n > 0)
                    .map(|(id, n)| format!("{}:{}", id.0, n))
                    .collect::<Vec<_>>()
                    .join("|");
                format!("{x},{y},{body}")
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    pub fn decode_chests(&mut self, raw: &str) -> bool {
        self.chests.clear();
        if raw.is_empty() {
            return true;
        }
        for part in raw.split(';').filter(|s| !s.is_empty()) {
            let mut it = part.splitn(3, ',');
            let (Some(xs), Some(ys), Some(body)) = (it.next(), it.next(), it.next()) else {
                continue;
            };
            let Ok(x) = xs.parse::<i32>() else {
                continue;
            };
            let Ok(y) = ys.parse::<i32>() else {
                continue;
            };
            let mut inv = HashMap::new();
            if !body.is_empty() {
                for e in body.split('|').filter(|s| !s.is_empty()) {
                    let Some((a, b)) = e.split_once(':') else {
                        continue;
                    };
                    let Ok(id) = a.parse::<u32>() else {
                        continue;
                    };
                    let Ok(n) = b.parse::<u32>() else {
                        continue;
                    };
                    if n > 0 {
                        inv.insert(ItemId(id), n);
                    }
                }
            }
            self.chests.insert((wrap_tx(x), y), inv);
        }
        true
    }

    /// 树苗剩余生长秒数（调试 / HUD）。
    pub fn sapling_left(&self, x: i32, y: i32) -> Option<f32> {
        self.grow_t.get(&(wrap_tx(x), y)).copied()
    }

    /// 剩余 HP；满血时返回 `max_hp`。
    pub fn hp_left(&self, x: i32, y: i32) -> u16 {
        let id = self.get(x, y);
        let max = id.max_hp();
        if max == 0 || max == u16::MAX {
            return max;
        }
        let x = wrap_tx(x);
        self.damage_hp.get(&(x, y)).copied().unwrap_or(max)
    }

    /// 损坏比例 0..=1（0 完好，1 将碎）。
    pub fn damage_ratio(&self, x: i32, y: i32) -> f32 {
        let id = self.get(x, y);
        let max = id.max_hp();
        if max == 0 || max == u16::MAX {
            return 0.0;
        }
        let left = self.hp_left(x, y);
        1.0 - (left as f32 / max as f32)
    }

    /// 对格造成伤害。打碎则置空气并返回原方块；否则 `None`。
    pub fn apply_damage(&mut self, x: i32, y: i32, dmg: u16) -> Option<BlockId> {
        if !self.y_in_bounds(y) || dmg == 0 {
            return None;
        }
        let id = self.get(x, y);
        if !id.mineable() {
            return None;
        }
        let max = id.max_hp();
        let x = wrap_tx(x);
        let key = (x, y);
        let left = self.damage_hp.get(&key).copied().unwrap_or(max);
        let next = left.saturating_sub(dmg);
        if next == 0 {
            self.damage_hp.remove(&key);
            // 走 set 以触发木箱掉落物与树苗清理
            self.set(x, y, BlockId::AIR);
            Some(id)
        } else {
            self.damage_hp.insert(key, next);
            None
        }
    }

    /// 在格中心生成地面掉落。
    pub fn spawn_drop_at_tile(&mut self, tx: i32, ty: i32, item: ItemId, count: u32) {
        if count == 0 {
            return;
        }
        let tx = wrap_tx(tx);
        let x = tx as f32 * TILE + TILE * 0.5 - 4.0;
        let y = ty as f32 * TILE + TILE * 0.35;
        self.drops.push(GroundDrop {
            x,
            y,
            item,
            count,
            bob: (hash2(self.seed, tx, ty) % 100) as f32 * 0.01,
        });
    }

    /// 掉落物轻浮与落地（贴最近实心顶）。
    pub fn tick_drops(&mut self, dt: f32) {
        // 先收集落地判定，避免边借 blocks 边改 drops
        let mut grounded_y: Vec<(usize, Option<f32>)> = Vec::with_capacity(self.drops.len());
        for (i, d) in self.drops.iter().enumerate() {
            let foot = d.y + 10.0;
            let tx = wrap_tx((d.x / TILE).floor() as i32);
            let ty = (foot / TILE).floor() as i32;
            let snap = if self.y_in_bounds(ty) && self.get(tx, ty).blocks_motion() {
                Some(ty as f32 * TILE - 10.0)
            } else {
                None
            };
            grounded_y.push((i, snap));
        }
        let max_y = WORLD_H as f32 * TILE - 12.0;
        for (i, snap) in grounded_y {
            let d = &mut self.drops[i];
            d.bob += dt;
            if let Some(y) = snap {
                d.y = y;
            } else {
                d.y += TILE * 6.0 * dt;
                if d.y > max_y {
                    d.y = max_y;
                }
            }
        }
    }

    /// 玩家碰撞盒附近拾取；返回拾取描述。
    pub fn try_pickup(&mut self, px: f32, py: f32, pw: f32, ph: f32) -> Vec<(ItemId, u32)> {
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let mut got = Vec::new();
        let mut i = 0;
        while i < self.drops.len() {
            let d = &self.drops[i];
            let dx = wrap_delta_x(cx, d.x + 4.0);
            let dy = (d.y + 5.0) - cy;
            let reach = TILE * 1.35;
            if dx.hypot(dy) <= reach {
                got.push((d.item, d.count));
                self.drops.swap_remove(i);
            } else {
                i += 1;
            }
        }
        got
    }

    pub fn has_block(&self, id: BlockId) -> bool {
        self.blocks.iter().any(|b| *b == id)
    }

    /// 玩家附近是否有工作台（制作站）。
    pub fn near_workbench(&self, px: f32, py: f32, pw: f32, ph: f32) -> bool {
        self.near_block(px, py, pw, ph, BlockId::WORKBENCH, 3)
    }

    pub fn near_furnace(&self, px: f32, py: f32, pw: f32, ph: f32) -> bool {
        self.near_block(px, py, pw, ph, BlockId::FURNACE, 3)
    }

    pub fn near_bed(&self, px: f32, py: f32, pw: f32, ph: f32) -> Option<(i32, i32)> {
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let tx = wrap_tx((cx / TILE).floor() as i32);
        let ty = (cy / TILE).floor() as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let x = wrap_tx(tx + dx);
                let y = ty + dy;
                if self.get(x, y) == BlockId::BED {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn near_block(&self, px: f32, py: f32, pw: f32, ph: f32, id: BlockId, r: i32) -> bool {
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let tx = wrap_tx((cx / TILE).floor() as i32);
        let ty = (cy / TILE).floor() as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if self.get(tx + dx, ty + dy) == id {
                    return true;
                }
            }
        }
        false
    }

    pub fn biome_at_x(&self, x: i32) -> BiomeId {
        biome_at(self.seed, wrap_tx(x))
    }

    pub fn surface_at(&self, x: i32) -> i32 {
        surface_height(self.seed, wrap_tx(x))
    }

    pub fn spawn_pos(&self) -> (f32, f32) {
        let tx = 26;
        let sh = surface_height(self.seed, tx);
        let feet_y = sh as f32 * TILE;
        let x = tx as f32 * TILE + (TILE - PLAYER_HIT_W) * 0.5;
        (x, feet_y - PLAYER_HIT_H)
    }

    /// 导出方块表（存档）。
    pub fn encode_blocks(&self) -> String {
        self.blocks
            .iter()
            .map(|b| b.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn encode_walls(&self) -> String {
        self.walls
            .iter()
            .map(|w| w.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// 稀疏导出水位（仅水格）。
    pub fn encode_fluids(&self) -> String {
        let mut parts = Vec::new();
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                if self.get(x, y) != BlockId::WATER {
                    continue;
                }
                let lv = self.fluid_level(x, y).0;
                parts.push(format!("{x},{y},{lv}"));
            }
        }
        parts.join(";")
    }

    pub fn decode_fluids(&mut self, raw: &str) -> bool {
        // 缺省：所有水格视为源
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                let idx = (y * WORLD_W + x) as usize;
                if self.blocks[idx] == BlockId::WATER {
                    self.fluid[idx] = FluidLevel::SOURCE;
                }
            }
        }
        if raw.is_empty() {
            return true;
        }
        for part in raw.split(';').filter(|s| !s.is_empty()) {
            let mut it = part.splitn(3, ',');
            let (Some(xs), Some(ys), Some(ls)) = (it.next(), it.next(), it.next()) else {
                continue;
            };
            let Ok(x) = xs.parse::<i32>() else {
                continue;
            };
            let Ok(y) = ys.parse::<i32>() else {
                continue;
            };
            let Ok(lv) = ls.parse::<u8>() else {
                continue;
            };
            if !self.y_in_bounds(y) {
                continue;
            }
            let x = wrap_tx(x);
            if self.get(x, y) == BlockId::WATER {
                self.fluid[Self::fluid_idx(x, y)] = FluidLevel(lv).clamp_valid();
            }
        }
        true
    }

    /// 从存档恢复方块表。
    pub fn decode_blocks(&mut self, raw: &str) -> bool {
        let expect = (WORLD_W * WORLD_H) as usize;
        let vals: Vec<u32> = raw
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        if vals.len() != expect {
            return false;
        }
        if self.fluid.len() != expect {
            self.fluid = vec![FluidLevel::SOURCE; expect];
        }
        for (i, v) in vals.into_iter().enumerate() {
            self.blocks[i] = BlockId(v);
            if self.blocks[i] != BlockId::WATER {
                self.fluid[i] = FluidLevel::SOURCE;
            }
        }
        self.damage_hp.clear();
        self.grow_t.clear();
        self.drops.clear();
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                if self.get(x, y) == BlockId::SAPLING {
                    let secs = 10.0 + (hash2(self.seed, x, y) % 90) as f32 * 0.1;
                    self.grow_t.insert((x, y), secs);
                }
            }
        }
        true
    }

    pub fn decode_walls(&mut self, raw: &str) -> bool {
        let expect = (WORLD_W * WORLD_H) as usize;
        let vals: Vec<u8> = raw
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        if vals.len() != expect {
            return false;
        }
        for (i, v) in vals.into_iter().enumerate() {
            self.walls[i] = WallId(v);
        }
        true
    }
}

/// 周期地表：用绕圆柱角度采样，缝处连续。
fn surface_height(seed: u64, x: i32) -> i32 {
    let x = wrap_tx(x);
    let ang = x as f32 / WORLD_W as f32 * TAU;
    let phase = (seed as f32 * 0.001).sin();
    let n1 = ((ang * 3.0 + phase).sin() * 4.0) as i32;
    let n2 = ((ang * 1.0 + phase * 0.5).cos() * 2.0) as i32;
    let base = WORLD_H / 2 + 8;
    (base + n1 + n2).clamp(12, WORLD_H - 8)
}

fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let x = wrap_tx(x) as u64;
    let mut v =
        seed ^ x.wrapping_mul(0x9E3779B97F4A7C15) ^ (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    v = (v ^ (v >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    v ^ (v >> 31)
}
