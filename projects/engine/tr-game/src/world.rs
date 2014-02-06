//! 侧视地表世界：确定性高度 + 方块表 + 地面掉落物。
//!
//! 坐标：`y = 0` 为世界顶（天空），`y` 增大向地心。
//! 有限边界（不回环）。逻辑格固定 16 像素。屏幕 2× 只走 [`DISPLAY_SCALE`]，不写进 `TILE`。

use std::collections::HashMap;
use tr_core::{BiomeId, BlockId, FluidLevel, ItemId, WallId, biome_at};

/// 约小世界规模宽度的 1/10，兼顾可玩宽度与生成耗时。
pub const WORLD_W: i32 = 420;
pub const WORLD_H: i32 = 180;
/// 出生列（世界中段）。
pub const SPAWN_TX: i32 = WORLD_W / 2;
/// 逻辑格边长（像素）。碰撞、速度和存档坐标都用这个值，不要改成屏幕放大后的尺寸。
pub const TILE: f32 = 16.0;
/// 整数显示倍率。相机把逻辑像素乘这个数得到屏幕像素。
pub const DISPLAY_SCALE: i32 = 2;

/// 逻辑坐标到屏幕像素：`(world - cam) * DISPLAY_SCALE`。
pub fn screen_of(world: f32, cam: f32) -> f32 {
    (world - cam) * DISPLAY_SCALE as f32
}

/// 逻辑长度到屏幕像素。
pub fn screen_len(logical: f32) -> f32 {
    logical * DISPLAY_SCALE as f32
}

/// 屏幕像素到逻辑坐标。
pub fn world_of_screen(screen: f32, cam: f32) -> f32 {
    cam + screen / DISPLAY_SCALE as f32
}

/// 屏幕宽度对应的逻辑视口宽度。
pub fn view_extent(screen_px: f32) -> f32 {
    screen_px / DISPLAY_SCALE as f32
}

/// 世界宽度（世界单位）。
pub fn world_pixel_w() -> f32 {
    WORLD_W as f32 * TILE
}

/// 世界高度（世界单位）。
pub fn world_pixel_h() -> f32 {
    WORLD_H as f32 * TILE
}

/// 格坐标 X 是否在世界内。
pub fn x_in_bounds(x: i32) -> bool {
    x >= 0 && x < WORLD_W
}

/// 格坐标 X 钳到 `[0, WORLD_W)`。仅用于已知需合法下标的路径。
pub fn wrap_tx(x: i32) -> i32 {
    x.clamp(0, WORLD_W - 1)
}

/// 世界坐标 X 钳到世界内（给碰撞盒左缘用，调用方再减宽度）。
pub fn wrap_xf(x: f32) -> f32 {
    x.clamp(0.0, (WORLD_W as f32 * TILE - 1.0).max(0.0))
}

/// 水平有符号距离（有限边界，不再跨缝取最短）。
pub fn wrap_delta_x(from: f32, to: f32) -> f32 {
    to - from
}

/// 方块左缘世界 X（有限边界，无周期像）。
pub fn tile_x_near(tx: i32, _near_x: f32) -> f32 {
    wrap_tx(tx) as f32 * TILE
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
    /// 稀疏帧。自然树和地形在生成 / 读档 / 编辑时写入，绘制只读。
    frames: HashMap<(i32, i32), (i16, i16)>,
    /// 背景墙帧。与前景帧分开，同一格可以同时有墙和方块。
    wall_frames: HashMap<(i32, i32), (i16, i16)>,
    /// 世界属性级地表基准。
    pub surface_level: i32,
    /// 世界属性级岩石层基准。
    pub rock_level: i32,
    /// 熔岩线（深层液体分界）。
    pub lava_line: i32,
    /// 每列地表 Y（生成剖面；读档后可按列扫描回填）。
    surface_y: Vec<i32>,
    /// 每列岩石层 Y。
    pub(crate) rock_y: Vec<i32>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        let n = (WORLD_W * WORLD_H) as usize;
        let mut w = Self {
            seed,
            blocks: vec![BlockId::AIR; n],
            walls: vec![WallId::NONE; n],
            fluid: vec![FluidLevel::SOURCE; n],
            damage_hp: HashMap::new(),
            wall_hp: HashMap::new(),
            grow_t: HashMap::new(),
            chests: HashMap::new(),
            drops: Vec::new(),
            frames: HashMap::new(),
            wall_frames: HashMap::new(),
            surface_level: WORLD_H / 4,
            rock_level: WORLD_H / 2,
            lava_line: (WORLD_H as f32 * 0.8) as i32,
            surface_y: vec![WORLD_H / 4; WORLD_W as usize],
            rock_y: vec![WORLD_H / 2; WORLD_W as usize],
        };

        let profile = crate::worldgen::generate_into(&mut w, seed);
        w.surface_level = profile.surface_level;
        w.rock_level = profile.rock_level;
        w.lava_line = profile.lava_line;
        w.surface_y = profile.surface_y;
        w.rock_y = profile.rock_y;

        // 出生点旁火把与补给箱
        let stx = wrap_tx(SPAWN_TX + 2);
        let sty = w.surface_at(stx) - 2;
        if w.get(stx, sty) == BlockId::AIR {
            w.set(stx, sty, BlockId::TORCH);
        }
        let ctx = wrap_tx(stx + 2);
        let cty = w.surface_at(ctx) - 1;
        if w.get(ctx, cty) == BlockId::AIR {
            w.set(ctx, cty, BlockId::CHEST);
            if let Some(chest) = w.chest_at(ctx, cty) {
                chest.insert(ItemId::GEL, 3);
                chest.insert(ItemId::WOOD, 6);
                chest.insert(ItemId::TORCH, 2);
                chest.insert(ItemId::LADDER, 8);
            }
        }

        crate::trees::stamp_frames(&mut w);
        crate::tile_frame::stamp_terrain_all(&mut w);
        crate::tile_frame::stamp_walls_all(&mut w);
        w
    }

    /// 生成期写格：不触发邻接 framing（整图结束后统一盖章）。
    pub(crate) fn set_raw(&mut self, x: i32, y: i32, id: BlockId) {
        if !self.in_bounds(x, y) {
            return;
        }
        let idx = (y * WORLD_W + x) as usize;
        self.blocks[idx] = id;
        self.damage_hp.remove(&(x, y));
        self.frames.remove(&(x, y));
        if id != BlockId::WATER {
            // 非水格清掉液量占位。
        }
        if id == BlockId::AIR {
            self.fluid[idx] = FluidLevel::SOURCE;
        }
    }

    /// 生成期写墙：不触发墙 framing。
    pub(crate) fn set_wall_raw(&mut self, x: i32, y: i32, id: WallId) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.walls[(y * WORLD_W + x) as usize] = id;
        self.wall_hp.remove(&(x, y));
        self.wall_frames.remove(&(x, y));
    }

    /// 生成期写液量。
    pub(crate) fn set_fluid(&mut self, x: i32, y: i32, level: FluidLevel) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.fluid[(y * WORLD_W + x) as usize] = level.clamp_valid();
    }

    /// 生成期清液量。
    pub(crate) fn clear_fluid(&mut self, x: i32, y: i32) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.fluid[(y * WORLD_W + x) as usize] = FluidLevel::SOURCE;
    }

    /// 一次性按过渡期水位规则烘焙流动/下落。
    ///
    /// **已隔离**：生成管线不再调用。保留实现仅供对照/删除前引用。
    #[allow(dead_code)]
    fn bake_fluid_flow(&mut self) {
        for _ in 0..48 {
            if !self.tick_fluids_once() {
                break;
            }
        }
    }

    /// 推进过渡期流体扩散 / 退水。
    ///
    /// **已隔离**：玩法循环不再调用。权威 `0..=255` 液量落地前勿重新挂回。
    #[allow(dead_code)]
    pub fn tick_fluids(&mut self, rounds: u32) {
        for _ in 0..rounds.max(1) {
            if !self.tick_fluids_once() {
                break;
            }
        }
    }

    /// 单轮流体步进（过渡期）。有变化返回 `true`。
    #[allow(dead_code)]
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
        if !self.in_bounds(x, y) {
            return;
        }
        let idx = (y * WORLD_W + x) as usize;
        if self.blocks[idx] == BlockId::WATER {
            self.blocks[idx] = BlockId::AIR;
            self.fluid[idx] = FluidLevel::SOURCE;
            self.frames.remove(&(x, y));
            crate::tile_frame::stamp_terrain_around(self, x, y);
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
        (y * WORLD_W + x) as usize
    }

    pub fn fluid_level(&self, x: i32, y: i32) -> FluidLevel {
        if !self.y_in_bounds(y) || self.get(x, y) != BlockId::WATER {
            return FluidLevel::SOURCE;
        }
        self.fluid[Self::fluid_idx(x, y)].clamp_valid()
    }

    pub fn set_water(&mut self, x: i32, y: i32, level: FluidLevel) {
        if !self.in_bounds(x, y) {
            return;
        }
        let idx = (y * WORLD_W + x) as usize;
        self.blocks[idx] = BlockId::WATER;
        self.fluid[idx] = level.clamp_valid();
        self.damage_hp.remove(&(x, y));
        self.grow_t.remove(&(x, y));
        self.frames.remove(&(x, y));
        crate::tile_frame::stamp_terrain_around(self, x, y);
    }

    /// 在地表草皮上长出一棵树（`surface_y` 为草皮格 Y）。
    pub fn grow_tree_at(&mut self, tx: i32, surface_y: i32, trunk: i32) {
        if !x_in_bounds(tx) {
            return;
        }
        let trunk = trunk.clamp(3, 8);
        // 清掉树苗
        self.grow_t.remove(&(tx, surface_y - 1));
        for i in 1..=trunk {
            let y = surface_y - i;
            if !self.y_in_bounds(y) {
                break;
            }
            let cur = self.get(tx, y);
            // 遇到实心格就停。不能跳过雪层再往上种，否则会在洞穴里留下没有土壤的树干。
            if !matches!(
                cur,
                BlockId::AIR | BlockId::LEAF | BlockId::SAPLING | BlockId::TREE
            ) {
                break;
            }
            self.set(tx, y, BlockId::TREE);
        }
        crate::trees::stamp_column(self, tx);
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
            let h = self.surface_at(x);
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

    /// 已写入的图集帧。没有记录表示绘制不得临场猜测。
    pub fn frame(&self, x: i32, y: i32) -> Option<(i16, i16)> {
        if !self.in_bounds(x, y) {
            return None;
        }
        self.frames.get(&(x, y)).copied()
    }

    /// 写入一格的图集帧。`set` 会清掉旧帧。
    pub fn set_frame(&mut self, x: i32, y: i32, frame_x: i16, frame_y: i16) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.frames.insert((x, y), (frame_x, frame_y));
    }

    /// 已写入的背景墙帧。
    pub fn wall_frame(&self, x: i32, y: i32) -> Option<(i16, i16)> {
        if !self.in_bounds(x, y) {
            return None;
        }
        self.wall_frames.get(&(x, y)).copied()
    }

    /// 写入背景墙帧。
    pub fn set_wall_frame(&mut self, x: i32, y: i32, frame_x: i16, frame_y: i16) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.wall_frames.insert((x, y), (frame_x, frame_y));
    }

    pub fn y_in_bounds(&self, y: i32) -> bool {
        y >= 0 && y < WORLD_H
    }

    /// 格是否在有限世界内。
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x_in_bounds(x) && self.y_in_bounds(y)
    }

    pub fn get(&self, x: i32, y: i32) -> BlockId {
        if !self.y_in_bounds(y) {
            return BlockId::STONE;
        }
        // 水平出界视为基岩墙，阻挡玩家离开地图。
        if !x_in_bounds(x) {
            return BlockId::STONE;
        }
        self.blocks[(y * WORLD_W + x) as usize]
    }

    pub fn get_wall(&self, x: i32, y: i32) -> WallId {
        if !self.in_bounds(x, y) {
            return WallId::NONE;
        }
        self.walls[(y * WORLD_W + x) as usize]
    }

    pub fn set_wall(&mut self, x: i32, y: i32, id: WallId) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.walls[(y * WORLD_W + x) as usize] = id;
        self.wall_hp.remove(&(x, y));
        self.wall_frames.remove(&(x, y));
        crate::tile_frame::stamp_walls_around(self, x, y);
    }

    /// 墙剩余 HP。
    pub fn wall_hp_left(&self, x: i32, y: i32) -> u16 {
        let id = self.get_wall(x, y);
        let max = id.max_hp();
        if max == 0 || !self.in_bounds(x, y) {
            return 0;
        }
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
        if !self.in_bounds(x, y) || dmg == 0 {
            return None;
        }
        let id = self.get_wall(x, y);
        if !id.mineable() {
            return None;
        }
        let max = id.max_hp();
        let key = (x, y);
        let left = self.wall_hp.get(&key).copied().unwrap_or(max);
        let next = left.saturating_sub(dmg);
        if next == 0 {
            self.wall_hp.remove(&key);
            self.walls[(y * WORLD_W + x) as usize] = WallId::NONE;
            self.wall_frames.remove(&key);
            crate::tile_frame::stamp_walls_around(self, x, y);
            Some(id)
        } else {
            self.wall_hp.insert(key, next);
            None
        }
    }

    pub fn set(&mut self, x: i32, y: i32, id: BlockId) {
        if !self.in_bounds(x, y) {
            return;
        }
        let prev = self.blocks[(y * WORLD_W + x) as usize];
        self.blocks[(y * WORLD_W + x) as usize] = id;
        self.damage_hp.remove(&(x, y));
        self.frames.remove(&(x, y));
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
        crate::tile_frame::stamp_terrain_around(self, x, y);
    }

    pub fn chest_at(&mut self, x: i32, y: i32) -> Option<&mut HashMap<ItemId, u32>> {
        if !self.in_bounds(x, y) || self.get(x, y) != BlockId::CHEST {
            return None;
        }
        Some(self.chests.entry((x, y)).or_default())
    }

    pub fn near_chest(&self, px: f32, py: f32, pw: f32, ph: f32) -> Option<(i32, i32)> {
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let tx = (cx / TILE).floor() as i32;
        let ty = (cy / TILE).floor() as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let x = tx + dx;
                let y = ty + dy;
                if self.in_bounds(x, y) && self.get(x, y) == BlockId::CHEST {
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
        let x = wrap_tx(x);
        if let Some(&y) = self.surface_y.get(x as usize) {
            return y;
        }
        // 旧存档无剖面时按列扫描首块实心。
        for y in 0..WORLD_H {
            if self.get(x, y).solid() {
                return y;
            }
        }
        self.surface_level
    }

    /// 按当前方块表重建每列地表剖面（读档后调用）。
    pub fn rebuild_surface_profile(&mut self) {
        if self.surface_y.len() != WORLD_W as usize {
            self.surface_y = vec![self.surface_level; WORLD_W as usize];
        }
        if self.rock_y.len() != WORLD_W as usize {
            self.rock_y = vec![self.rock_level; WORLD_W as usize];
        }
        let mut sum_s = 0i64;
        let mut sum_r = 0i64;
        for x in 0..WORLD_W {
            let mut sy = self.surface_level;
            for y in 0..WORLD_H {
                if self.get(x, y).solid() {
                    sy = y;
                    break;
                }
            }
            self.surface_y[x as usize] = sy;
            // 岩石层：地表以下首块石头或深度兜底。
            let mut ry = (sy + ((WORLD_H - sy) * 2) / 5).min(WORLD_H - 1);
            for y in sy..WORLD_H {
                if self.get(x, y) == BlockId::STONE {
                    ry = y;
                    break;
                }
            }
            self.rock_y[x as usize] = ry.max(sy + 2);
            sum_s += sy as i64;
            sum_r += self.rock_y[x as usize] as i64;
        }
        self.surface_level = (sum_s / WORLD_W as i64) as i32;
        self.rock_level = (sum_r / WORLD_W as i64) as i32;
        self.lava_line = ((WORLD_H as f32 * 0.8) as i32).min(WORLD_H - 8);
    }

    pub fn spawn_pos(&self) -> (f32, f32) {
        let tx = SPAWN_TX;
        let sh = self.surface_at(tx);
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
    ///
    /// `fixture_ids` 为真时按旧夹具编号迁移；否则数值已是当前身份。
    pub fn decode_blocks(&mut self, raw: &str, fixture_ids: bool) -> bool {
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
            self.blocks[i] = if fixture_ids {
                BlockId::from_fixture_id(v)
            } else {
                BlockId(v)
            };
            if self.blocks[i] != BlockId::WATER {
                self.fluid[i] = FluidLevel::SOURCE;
            }
        }
        self.damage_hp.clear();
        self.grow_t.clear();
        self.drops.clear();
        self.rebuild_surface_profile();
        self.promote_living_trees();
        self.frames.clear();
        crate::trees::stamp_frames(self);
        crate::tile_frame::stamp_terrain_all(self);
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

    /// 旧存档：地表上的实心 `WOOD` 树干升为可穿越 `TREE`，并清掉假 `LEAF` 树冠格。
    fn promote_living_trees(&mut self) {
        for x in 0..WORLD_W {
            let mut y = 1;
            while y < WORLD_H - 1 {
                if self.get(x, y) != BlockId::WOOD {
                    y += 1;
                    continue;
                }
                let top = y;
                while y < WORLD_H && self.get(x, y) == BlockId::WOOD {
                    y += 1;
                }
                let height = y - top;
                if height < 3 {
                    continue;
                }
                let soil = self.get(x, y);
                if !matches!(
                    soil,
                    BlockId::GRASS | BlockId::DIRT | BlockId::SNOW | BlockId::SAND
                ) {
                    continue;
                }
                for ty in top..y {
                    self.set(x, ty, BlockId::TREE);
                }
                // 清掉旧假叶冠占位。
                for dy in -3i32..=2 {
                    for dx in -3i32..=3 {
                        let lx = wrap_tx(x + dx);
                        let ly = top + dy;
                        if self.y_in_bounds(ly) && self.get(lx, ly) == BlockId::LEAF {
                            self.set(lx, ly, BlockId::AIR);
                        }
                    }
                }
            }
        }
        // 已是 TREE 的柱旁残留假叶也清掉。
        for x in 0..WORLD_W {
            for y in 0..WORLD_H {
                if self.get(x, y) != BlockId::LEAF {
                    continue;
                }
                let mut near_tree = false;
                for dx in -2..=2 {
                    for dy in -2..=2 {
                        if self.get(wrap_tx(x + dx), y + dy) == BlockId::TREE {
                            near_tree = true;
                        }
                    }
                }
                if near_tree {
                    self.set(x, y, BlockId::AIR);
                }
            }
        }
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
        self.wall_frames.clear();
        crate::tile_frame::stamp_walls_all(self);
        true
    }
}

/// 有限边界哈希（生长 / 草皮抽样用）。
fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let x = x.clamp(0, WORLD_W) as u64;
    let mut v =
        seed ^ x.wrapping_mul(0x9E3779B97F4A7C15) ^ (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    v = (v ^ (v >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    v ^ (v >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_tile_is_16_and_display_is_integer_2x() {
        assert_eq!(TILE, 16.0);
        assert_eq!(DISPLAY_SCALE, 2);
        assert_eq!(screen_len(TILE), 32.0);
        assert_eq!(screen_of(TILE, 0.0), 32.0);
        assert_eq!(world_of_screen(32.0, 0.0), TILE);
        assert_eq!(view_extent(1280.0), 640.0);
        let back = world_of_screen(screen_of(48.0, 10.0), 10.0);
        assert!((back - 48.0).abs() < 1e-4);
    }

    #[test]
    fn living_tree_has_no_fake_leaf_crown() {
        let world = World::generate(7);
        let mut trees = 0;
        let mut leaves = 0;
        for x in 0..WORLD_W {
            for y in 0..WORLD_H {
                match world.get(x, y) {
                    BlockId::TREE => trees += 1,
                    BlockId::LEAF => leaves += 1,
                    _ => {}
                }
            }
        }
        assert!(trees > 0, "生成世界应有树干");
        assert_eq!(leaves, 0, "树冠不得占用假 LEAF 格");
        assert!(!BlockId::TREE.blocks_motion());
        assert!(!BlockId::TREE.solid());
        assert_eq!(BlockId::TREE.drop_item(), Some(ItemId::WOOD));
        assert!(BlockId::WOOD.blocks_motion());
        assert_eq!(BlockId::TREES.0, 5, "须对齐正版 TileID.Trees");
        assert_eq!(BlockId::DIRT.0, 0, "须对齐正版 TileID.Dirt");
        assert_eq!(BlockId::WOOD.0, 30, "须对齐正版 TileID.WoodBlock");
        assert_eq!(BlockId::AIR.0, u32::MAX, "空气不是类型 0");
        assert!(BlockId::TREES.is_tree());
        assert_eq!(BlockId::from_fixture_id(1), BlockId::DIRT);
        assert_eq!(BlockId::from_fixture_id(0), BlockId::AIR);
        assert_eq!(BlockId::from_fixture_id(6), BlockId::WOOD);
    }

    #[test]
    fn old_wood_trunk_promotes_to_passable_tree() {
        let mut world = World::generate(11);
        let mut sample = None;
        'scan: for x in 0..WORLD_W {
            for y in 1..WORLD_H - 1 {
                if world.get(x, y) == BlockId::TREE {
                    sample = Some((x, y));
                    break 'scan;
                }
            }
        }
        let (x, y) = sample.expect("应有树");
        let mut top = y;
        while top > 1 && world.get(x, top - 1) == BlockId::TREE {
            top -= 1;
        }
        let mut bot = y;
        while bot + 1 < WORLD_H && world.get(x, bot + 1) == BlockId::TREE {
            bot += 1;
        }
        for ty in top..=bot {
            world.set(x, ty, BlockId::WOOD);
        }
        world.promote_living_trees();
        assert_eq!(world.get(x, y), BlockId::TREE);
        assert!(!world.get(x, y).blocks_motion());

        world.set(4, WORLD_H - 3, BlockId::WOOD);
        world.promote_living_trees();
        assert_eq!(world.get(4, WORLD_H - 3), BlockId::WOOD);
    }

    #[test]
    fn layered_terrain_has_dirt_over_stone_and_caves() {
        let world = World::generate(42);
        assert!(world.surface_level > 8);
        assert!(world.rock_level > world.surface_level + 4);
        assert!(world.lava_line > world.rock_level);

        let mut dirt = 0u32;
        let mut stone = 0u32;
        let mut air_below = 0u32;
        let mut copper = 0u32;
        let mut iron = 0u32;
        let mut sand = 0u32;
        for x in 0..WORLD_W {
            let sy = world.surface_at(x);
            let ry = world.rock_y[x as usize];
            assert!(ry > sy, "列 {x}: rock {ry} 应低于 surface {sy}");
            for y in sy..ry.min(sy + 8) {
                let id = world.get(x, y);
                if id == BlockId::DIRT || id == BlockId::GRASS {
                    dirt += 1;
                }
            }
            for y in ry..world.lava_line.min(ry + 20) {
                match world.get(x, y) {
                    BlockId::STONE => stone += 1,
                    BlockId::AIR | BlockId::WATER => air_below += 1,
                    BlockId::COPPER_ORE => copper += 1,
                    BlockId::IRON_ORE => iron += 1,
                    BlockId::SAND => sand += 1,
                    _ => {}
                }
            }
        }
        assert!(dirt > 100, "表层应有泥土/草: {dirt}");
        assert!(stone > 200, "岩石层应有石头: {stone}");
        assert!(air_below > 20, "地下应有洞穴空气: {air_below}");
        assert!(copper + iron > 5, "应有铜/铁矿脉: cu={copper} fe={iron}");
        assert!(sand > 0, "沙漠或海底应有沙: {sand}");
        let mut ocean = 0u32;
        let shore = (WORLD_W as f64 * 0.12) as i32;
        for x in 1..shore {
            for y in 0..WORLD_H {
                if world.get(x, y) == BlockId::WATER {
                    ocean += 1;
                }
            }
        }
        assert!(ocean > 10, "左侧海洋应有水: {ocean}");
        let mut wide = 0u32;
        for x in 1..WORLD_W - 1 {
            for y in world.rock_level..world.lava_line {
                if world.get(x, y) == BlockId::AIR
                    && (world.get(x - 1, y) == BlockId::AIR || world.get(x + 1, y) == BlockId::AIR)
                {
                    wide += 1;
                }
            }
        }
        assert!(wide > 5, "洞穴应横向展开: {wide}");
    }

    #[test]
    fn tree_frames_are_stored_with_the_trunk() {
        let mut world = World::generate(7);
        let mut sample = None;
        for x in 0..WORLD_W {
            for y in 0..WORLD_H {
                if world.get(x, y) != BlockId::TREE {
                    continue;
                }
                let (fx, fy) = world.frame(x, y).expect("树干必须带已写入的帧");
                let row = fy & 255;
                assert!(fx >= 0);
                assert!(row == 0 || row == 22 || row == 44, "行 {row}");
                sample = Some((x, y));
            }
        }
        let (x, y) = sample.expect("应有树");
        world.set(x, y, BlockId::AIR);
        assert!(world.frame(x, y).is_none());
    }

    #[test]
    fn terrain_frames_are_stored_and_follow_edits() {
        let mut world = World::generate(4);
        let mut dirt = None;
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                let id = world.get(x, y);
                if !matches!(
                    id,
                    BlockId::DIRT
                        | BlockId::GRASS
                        | BlockId::STONE
                        | BlockId::SAND
                        | BlockId::SNOW
                        | BlockId::COPPER_ORE
                        | BlockId::IRON_ORE
                ) {
                    continue;
                }
                let stored = world.frame(x, y).expect("地形必须带已写入的帧");
                let live = crate::tile_frame::frame_uv_px(&world, x, y).expect("规则能算出帧");
                assert_eq!((stored.0 as u16, stored.1 as u16), live);
                if dirt.is_none() && id == BlockId::DIRT && y > 0 {
                    dirt = Some((x, y));
                }
            }
        }
        let (x, y) = dirt.expect("应有泥土");
        world.set(x, y - 1, BlockId::STONE);
        let stored = world.frame(x, y).expect("改邻居后泥土帧仍在");
        let live = crate::tile_frame::frame_uv_px(&world, x, y).unwrap();
        assert_eq!((stored.0 as u16, stored.1 as u16), live);
    }

    #[test]
    fn wall_frames_are_stored_and_follow_edits() {
        let mut world = World::generate(5);
        let mut sample = None;
        for y in 0..WORLD_H {
            for x in 0..WORLD_W {
                if world.get_wall(x, y) == tr_core::WallId::NONE {
                    continue;
                }
                let stored = world.wall_frame(x, y).expect("墙必须带已写入的帧");
                let live = crate::tile_frame::wall_frame_uv_px(&world, x, y).expect("规则能算出墙帧");
                assert_eq!((stored.0 as u16, stored.1 as u16), live);
                if sample.is_none() {
                    sample = Some((x, y));
                }
            }
        }
        let (x, y) = sample.expect("应有背景墙");
        world.set_wall(x, y, tr_core::WallId::NONE);
        assert!(world.wall_frame(x, y).is_none());
        if world.in_bounds(x + 1, y) && world.get_wall(x + 1, y) != tr_core::WallId::NONE {
            let stored = world.wall_frame(x + 1, y).unwrap();
            let live = crate::tile_frame::wall_frame_uv_px(&world, x + 1, y).unwrap();
            assert_eq!((stored.0 as u16, stored.1 as u16), live);
        }
    }
}
