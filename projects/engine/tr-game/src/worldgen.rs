//! 世界生成。
//!
//! 地表从高度约 30% 出发，再乘一段 0.45–0.55 的系数，随后按特征段游走。
//! 合法高度是 17%–30%。碰到边界就结束当前特征段。两侧海岸若深过 25% 就拉回。
//! 岩石层跟在地表下方，间距约 5%–35% 高度。世界地表属性取最深地表再加一小段。
//! 团块是曼哈顿菱形漫游。步数按小世界 4200×1200 的绝对格数写成，再按当前高度缩短，
//! 避免短图被长洞穴挖穿。截面强度仍是绝对格数。次数按面积或宽度取整。

use tr_core::{BlockId, FluidLevel, WallId};

use crate::world::{WORLD_H, WORLD_W, World};

/// 算法里的绝对格数是按小世界 4200×1200 写的。
const REF_W: f64 = 4200.0;
const REF_H: f64 = 1200.0;

/// 生成结果的层与剖面，写入 [`World`] 后供 `surface_at` 使用。
pub struct GenProfile {
    /// 世界属性级地表基准。
    pub surface_level: i32,
    /// 世界属性级岩石层基准。
    pub rock_level: i32,
    /// 熔岩线（约高度 80%）。
    pub lava_line: i32,
    /// 每列地表 Y（空气与实心交界，格在此及以下为实心层起点）。
    pub surface_y: Vec<i32>,
    /// 每列岩石层 Y。
    pub rock_y: Vec<i32>,
}

/// 确定性生成用 PRNG（splitmix64 变体）。
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0xA5A5_5A5A_C3C3_3C3C,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `[lo, hi)`；若区间空则返回 `lo`。
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }

    fn chance(&mut self, n: u32) -> bool {
        n > 0 && (self.next_u64() % n as u64) == 0
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

fn floor_ratio(n: f64) -> i32 {
    n as i32
}

/// 把小世界高度上的绝对格数缩到当前高度。正数至少 1，负数至多 -1。
fn scale_h(n: i32) -> i32 {
    scale_axis(n, WORLD_H as f64 / REF_H)
}

/// 把小世界宽度上的绝对格数缩到当前宽度。
fn scale_w(n: i32) -> i32 {
    scale_axis(n, WORLD_W as f64 / REF_W)
}

fn scale_axis(n: i32, factor: f64) -> i32 {
    if n == 0 {
        return 0;
    }
    let v = (n as f64 * factor).round() as i32;
    if n > 0 { v.max(1) } else { v.min(-1) }
}

struct Terrain {
    surface_y: Vec<i32>,
    rock_y: Vec<i32>,
    surface_low: i32,
    surface_high: i32,
    rock_low: i32,
    rock_high: i32,
    surface_level: i32,
    rock_level: i32,
    lava_line: i32,
}

/// 在空世界上跑完整开局生成管线（不含树帧 / 地形 framing）。
pub fn generate_into(world: &mut World, seed: u64) -> GenProfile {
    let mut rng = Rng::new(seed);
    let area = (WORLD_W * WORLD_H) as f64;

    let terrain = terrain_profile(&mut rng);
    fill_columns(world, &terrain.surface_y, &terrain.rock_y, terrain.lava_line);
    place_walls(world, &terrain.surface_y, &terrain.rock_y, terrain.lava_line);

    let surface_low = terrain.surface_low;
    let surface_high = terrain.surface_high.max(surface_low + 1);
    let rock_low = terrain.rock_low;
    let rock_high = terrain.rock_high.max(rock_low + 1);

    scatter_layers(world, &mut rng, area, surface_low, rock_high, rock_low);
    carve_small_holes(world, &mut rng, area, surface_high);
    carve_dirt_caves(world, &mut rng, area, surface_low, rock_high);
    carve_rock_caves(world, &mut rng, area, rock_high);
    carve_surface_caves(world, &mut rng, &terrain.surface_y, terrain.surface_level);
    spread_grass(world, &terrain.surface_y);
    place_ores(
        world,
        &mut rng,
        area,
        surface_low,
        surface_high,
        rock_low,
        rock_high,
    );
    place_lakes(world, &mut rng, terrain.rock_level, terrain.lava_line);
    place_underworld(world, &mut rng, terrain.lava_line);
    place_oceans(world, &mut rng, &terrain.surface_y);
    let mean_surface = terrain.surface_y.iter().sum::<i32>() / WORLD_W.max(1);
    place_snow(world, &mut rng, mean_surface, terrain.rock_level);
    place_desert(world, &mut rng, mean_surface, terrain.lava_line);
    plant_trees(world, &mut rng, &terrain.surface_y);

    GenProfile {
        surface_level: terrain.surface_level,
        rock_level: terrain.rock_level,
        lava_line: terrain.lava_line,
        surface_y: terrain.surface_y,
        rock_y: terrain.rock_y,
    }
}

/// 特征段地表：高原 / 丘 / 谷 / 山 / 洼。高度夹在 17%–30%。海岸过深则拉回 25%。
fn terrain_profile(rng: &mut Rng) -> Terrain {
    let h = WORLD_H as f64;
    let mut cur_surface = h * 0.3 * rng.range(90, 110) as f64 * 0.005;
    let mut cur_rock = (cur_surface + h * 0.2) * rng.range(90, 110) as f64 * 0.01;
    let beach = scale_w(275).max(4);
    let beach_right = WORLD_W - beach;

    let mut surface_y = vec![0; WORLD_W as usize];
    let mut rock_y = vec![0; WORLD_W as usize];
    let mut surface_low = i32::MAX;
    let mut surface_high = i32::MIN;
    let mut rock_low = i32::MAX;
    let mut rock_high = i32::MIN;

    let mut feature = 0;
    let mut feature_left = 0;

    for x in 0..WORLD_W {
        if feature_left <= 0 {
            feature = rng.range(0, 5);
            let mut left = rng.range(5, 40);
            if feature == 0 {
                left = (left as f64 * rng.range(5, 30) as f64 * 0.2) as i32;
            }
            feature_left = if left <= 0 { 0 } else { scale_w(left) };
        }
        feature_left -= 1;

        cur_surface += surface_offset(rng, feature) as f64;
        if cur_surface < h * 0.17 {
            cur_surface = h * 0.17;
            feature_left = 0;
        } else if cur_surface > h * 0.3 {
            cur_surface = h * 0.3;
            feature_left = 0;
        }
        if (x < beach || x > beach_right) && cur_surface > h * 0.25 {
            cur_surface = h * 0.25;
            feature_left = 1;
        }

        let sy = cur_surface.floor() as i32;
        surface_y[x as usize] = sy;
        surface_low = surface_low.min(sy);
        surface_high = surface_high.max(sy);

        while rng.chance(3) {
            cur_rock += rng.range(-2, 3) as f64;
        }
        let min_rock = cur_surface + h * 0.05;
        let max_rock = cur_surface + h * 0.35;
        if cur_rock < min_rock {
            cur_rock += 1.0;
        }
        if cur_rock > max_rock {
            cur_rock -= 1.0;
        }
        let stored_rock = (cur_rock.floor() as i32).max(sy + 1);
        rock_y[x as usize] = stored_rock;
        rock_low = rock_low.min(stored_rock);
        rock_high = rock_high.max(stored_rock);
    }

    let surface_level = (surface_high + scale_h(25)).min(WORLD_H - 8);
    let quantum = scale_h(6);
    let span = ((rock_high - surface_level) / quantum) * quantum;
    let mut rock_level = surface_level + span;
    if rock_level <= surface_level + 4 {
        rock_level = surface_level + quantum.max(6);
    }
    rock_level = rock_level.min(WORLD_H - 16);

    let water = ((rock_level as f64 + h) * 0.5) as i32 + rng.range(scale_h(-100), scale_h(20).max(1));
    let lava_line = (water + rng.range(scale_h(50), scale_h(80).max(scale_h(50) + 1)))
        .clamp(rock_level + 6, WORLD_H - 8);

    Terrain {
        surface_y,
        rock_y,
        surface_low,
        surface_high,
        rock_low,
        rock_high,
        surface_level,
        rock_level,
        lava_line,
    }
}

fn surface_offset(rng: &mut Rng, feature: i32) -> i32 {
    let mut off = 0;
    match feature {
        0 => {
            while rng.chance(7) {
                off += rng.range(-1, 2);
            }
        }
        1 => {
            while rng.chance(4) {
                off -= 1;
            }
            while rng.chance(10) {
                off += 1;
            }
        }
        2 => {
            while rng.chance(4) {
                off += 1;
            }
            while rng.chance(10) {
                off -= 1;
            }
        }
        3 => {
            while rng.chance(2) {
                off -= 1;
            }
            while rng.chance(6) {
                off += 1;
            }
        }
        _ => {
            while rng.chance(2) {
                off += 1;
            }
            while rng.chance(5) {
                off -= 1;
            }
        }
    }
    off
}

fn fill_columns(world: &mut World, surface_y: &[i32], rock_y: &[i32], lava_line: i32) {
    for x in 0..WORLD_W {
        let sy = surface_y[x as usize];
        let ry = rock_y[x as usize].max(sy);
        for y in 0..WORLD_H {
            let id = if y < sy {
                BlockId::AIR
            } else if y < ry {
                BlockId::DIRT
            } else if y < lava_line {
                BlockId::STONE
            } else {
                BlockId::AIR
            };
            world.set_raw(x, y, id);
        }
    }
}

fn place_walls(world: &mut World, surface_y: &[i32], rock_y: &[i32], lava_line: i32) {
    for x in 0..WORLD_W {
        let sy = surface_y[x as usize];
        let ry = rock_y[x as usize].max(sy);
        let last = lava_line.min(WORLD_H - 1);
        if sy > last {
            continue;
        }
        for y in sy..=last {
            let wall = if y < ry { WallId::DIRT } else { WallId::STONE };
            world.set_wall_raw(x, y, wall);
        }
    }
}

fn scatter_layers(
    world: &mut World,
    rng: &mut Rng,
    area: f64,
    surface_low: i32,
    rock_high: i32,
    rock_low: i32,
) {
    let rocks_in_dirt = floor_ratio(area * 1.5e-4);
    for _ in 0..rocks_in_dirt {
        let px = rng.range(10, WORLD_W - 10);
        let py = rng.range(surface_low, rock_high.max(surface_low + 1));
        wander(world, rng, px, py, 4, 10, 5, 30, Some(BlockId::STONE), 0.0, 0.0);
    }
    let dirt_in_rocks = floor_ratio(area * 1.5e-4);
    for _ in 0..dirt_in_rocks {
        let px = rng.range(10, WORLD_W - 10);
        let py = rng.range(rock_low, (WORLD_H - 40).max(rock_low + 1));
        wander(world, rng, px, py, 4, 10, 5, 30, Some(BlockId::DIRT), 0.0, 0.0);
    }
}

fn carve_small_holes(world: &mut World, rng: &mut Rng, area: f64, surface_high: i32) {
    let count = floor_ratio(area * 0.0015);
    let y1 = (WORLD_H - 20).max(surface_high + 1);
    for _ in 0..count {
        let tx = rng.range(10, WORLD_W - 10);
        let ty = rng.range(surface_high, y1);
        wander(world, rng, tx, ty, 2, 5, 2, 20, None, 0.0, 0.0);
        wander(world, rng, tx, ty, 8, 15, 7, 30, None, 0.0, 0.0);
    }
}

fn carve_dirt_caves(
    world: &mut World,
    rng: &mut Rng,
    area: f64,
    surface_low: i32,
    rock_high: i32,
) {
    let count = floor_ratio(area * 3e-5);
    for _ in 0..count {
        let tx = rng.range(10, WORLD_W - 10);
        let ty = rng.range(surface_low, rock_high.max(surface_low + 1));
        wander(world, rng, tx, ty, 5, 15, 30, 200, None, 0.0, 0.0);
    }
}

fn carve_rock_caves(world: &mut World, rng: &mut Rng, area: f64, rock_high: i32) {
    let count = floor_ratio(area * 0.00013);
    let y1 = (WORLD_H - 20).max(rock_high + 1);
    for _ in 0..count {
        let tx = rng.range(10, WORLD_W - 10);
        let ty = rng.range(rock_high, y1);
        wander(world, rng, tx, ty, 6, 20, 50, 300, None, 0.0, 0.0);
    }
}

fn carve_surface_caves(world: &mut World, rng: &mut Rng, surface_y: &[i32], base_surface: i32) {
    let beach_left = floor_ratio(WORLD_W as f64 * 0.08);
    let beach_right = WORLD_W - beach_left;
    let span_lo = beach_left;
    let span_hi = beach_right.max(beach_left + 1);
    let surf_small = floor_ratio(WORLD_W as f64 * 0.002);
    let surf_medium = floor_ratio(WORLD_W as f64 * 0.0007);
    let surf_large = floor_ratio(WORLD_W as f64 * 0.0003);
    let surf_horiz = floor_ratio(WORLD_W as f64 * 0.0004);

    for _ in 0..surf_small {
        let tx = rng.range(span_lo, span_hi);
        let sy = surface_y.get(tx as usize).copied().unwrap_or(base_surface);
        let drift = rng.unit() * 0.1;
        wander(world, rng, tx, sy, 3, 6, 5, 50, None, drift, 1.0);
    }
    for _ in 0..surf_medium {
        let tx = rng.range(span_lo, span_hi);
        let sy = surface_y.get(tx as usize).copied().unwrap_or(base_surface);
        let drift = rng.unit() * 0.1;
        wander(world, rng, tx, sy, 10, 15, 50, 130, None, drift, 2.0);
    }
    for _ in 0..surf_large {
        let tx = rng.range(span_lo, span_hi);
        let sy = surface_y.get(tx as usize).copied().unwrap_or(base_surface);
        wander(world, rng, tx, sy, 12, 25, 150, 500, None, 0.0, 4.0);
        wander(world, rng, tx, sy, 8, 17, 60, 200, None, 0.0, 2.0);
        wander(world, rng, tx, sy, 5, 13, 40, 170, None, 0.0, 2.0);
    }
    for _ in 0..surf_horiz {
        let tx = rng.range(span_lo, span_hi);
        let sy = surface_y.get(tx as usize).copied().unwrap_or(base_surface) + rng.range(10, 40);
        wander(world, rng, tx, sy, 7, 12, 150, 250, None, 0.0, 1.0);
    }
}

fn spread_grass(world: &mut World, surface_y: &[i32]) {
    for x in 0..WORLD_W {
        let sy = surface_y[x as usize];
        let start = (sy - 5).max(0);
        let end = (sy + 30).min(WORLD_H);
        for y in start..end {
            if world.get(x, y) != BlockId::DIRT {
                continue;
            }
            let exposed = y == 0 || world.get(x, y - 1) == BlockId::AIR;
            if exposed {
                world.set_raw(x, y, BlockId::GRASS);
            }
            break;
        }
    }
}

fn place_ores(
    world: &mut World,
    rng: &mut Rng,
    area: f64,
    surface_low: i32,
    surface_high: i32,
    rock_low: i32,
    rock_high: i32,
) {
    struct Zone {
        tile: BlockId,
        mult: f64,
        y0: i32,
        y1: i32,
        s_min: i32,
        s_max: i32,
        st_min: i32,
        st_max: i32,
    }
    let deep = (WORLD_H - 20).max(rock_low + 1);
    // 银、金没有对应方块，不在这里用铜铁冒充。
    let zones = [
        Zone { tile: BlockId::COPPER_ORE, mult: 6e-5, y0: surface_low, y1: surface_high, s_min: 3, s_max: 6, st_min: 2, st_max: 6 },
        Zone { tile: BlockId::COPPER_ORE, mult: 8e-5, y0: surface_high, y1: rock_high, s_min: 3, s_max: 7, st_min: 3, st_max: 7 },
        Zone { tile: BlockId::COPPER_ORE, mult: 2e-4, y0: rock_low, y1: deep, s_min: 4, s_max: 9, st_min: 4, st_max: 8 },
        Zone { tile: BlockId::IRON_ORE, mult: 3e-5, y0: surface_low, y1: surface_high, s_min: 3, s_max: 7, st_min: 2, st_max: 5 },
        Zone { tile: BlockId::IRON_ORE, mult: 8e-5, y0: surface_high, y1: rock_high, s_min: 3, s_max: 6, st_min: 3, st_max: 6 },
        Zone { tile: BlockId::IRON_ORE, mult: 2e-4, y0: rock_low, y1: deep, s_min: 4, s_max: 9, st_min: 4, st_max: 8 },
    ];
    for z in zones {
        let count = floor_ratio(area * z.mult);
        let y1 = z.y1.max(z.y0 + 1);
        for _ in 0..count {
            let ox = rng.range(10, WORLD_W - 10);
            let oy = rng.range(z.y0, y1);
            let strength = rng.range(z.s_min, z.s_max) as f64;
            let steps = rng.range(z.st_min, z.st_max);
            tile_runner(world, rng, ox, oy, strength, steps, Some(z.tile), 0.0, 0.0);
        }
    }
}

fn place_lakes(world: &mut World, rng: &mut Rng, base_rock: i32, lava_line: i32) {
    let lake_count = floor_ratio(WORLD_W as f64 * 0.01);
    let x0 = 50.min(WORLD_W / 4);
    let x1 = (WORLD_W - 50).max(x0 + 1);
    for _ in 0..lake_count {
        let lx = rng.range(x0, x1);
        let ly = rng.range(base_rock, lava_line.max(base_rock + 1));
        let strength = 0.8 + rng.unit() * 0.8;
        lake_at(world, rng, lx, ly, strength);
    }
}

fn lake_at(world: &mut World, rng: &mut Rng, x: i32, y: i32, strength: f64) {
    let sy = WORLD_H as f64 / REF_H;
    let mut num1 = rng.range(25, 50) as f64 * strength * sy;
    let mut num3 = rng.range(30, 80) as f64 * sy;
    if rng.chance(5) {
        num1 *= 1.5;
        num3 *= 1.2;
    }
    let mut pos_x = x as f64;
    let mut pos_y = y as f64 - num3 * 0.3;
    let mut vel_x = rng.range(-10, 11) as f64 * 0.1;
    let mut vel_y = rng.range(-20, -10) as f64 * 0.1;

    let mut min_x = WORLD_W;
    let mut max_x = 0;
    let mut min_y = WORLD_H;
    let mut max_y = 0;

    while num1 > 0.0 && num3 > 0.0 {
        num1 -= rng.range(0, 3) as f64;
        num3 -= 1.0;
        let num2 = num1 * rng.range(80, 121) as f64 * 0.01;
        let x0 = box_lo(pos_x - num1 * 0.5);
        let x1 = box_hi(pos_x + num1 * 0.5, WORLD_W - 1);
        let y0 = box_lo(pos_y - num1 * 0.5);
        let y1 = box_hi(pos_y + num1 * 0.5, WORLD_H - 1);
        for tx in x0..x1 {
            for ty in y0..y1 {
                let dx = tx as f64 - pos_x;
                let dy = ty as f64 - pos_y;
                if (dx * dx + dy * dy).sqrt() < num2 * 0.4 {
                    world.set_raw(tx, ty, BlockId::AIR);
                    world.clear_fluid(tx, ty);
                    min_x = min_x.min(tx);
                    max_x = max_x.max(tx);
                    min_y = min_y.min(ty);
                    max_y = max_y.max(ty);
                }
            }
        }
        pos_x += vel_x;
        pos_y += vel_y;
        vel_x += rng.range(-10, 11) as f64 * 0.05;
        vel_y += rng.range(-10, 11) as f64 * 0.05;
        vel_x = vel_x.clamp(-0.5, 0.5);
        vel_y = vel_y.clamp(0.5, 1.5);
    }
    if min_x > max_x {
        return;
    }
    let cavity_h = max_y - min_y;
    let water_line = min_y + (cavity_h as f64 * 0.2) as i32;
    for tx in min_x..=max_x {
        for ty in (min_y..=max_y).rev() {
            if world.get(tx, ty).solid() {
                continue;
            }
            if ty > water_line {
                world.set_raw(tx, ty, BlockId::WATER);
                world.set_fluid(tx, ty, FluidLevel::SOURCE);
            }
        }
    }
}

/// 深层：顶板随机游走，实心填到接近底边，再挖水平洞穴和向上的烟囱。
/// 没有灰烬和熔岩，实心用石头，不灌岩浆，也不放狱石。
fn place_underworld(world: &mut World, rng: &mut Rng, lava_line: i32) {
    let mut ceil_walk = lava_line as f64;
    let mut ceiling = vec![lava_line; WORLD_W as usize];
    for col in 0..WORLD_W {
        ceil_walk += (rng.unit() - 0.5) * scale_h(6) as f64;
        let amp = scale_h(30) as f64;
        ceil_walk = ceil_walk.clamp(lava_line as f64 - amp, lava_line as f64 + amp);
        ceiling[col as usize] = ceil_walk.floor() as i32;
    }

    let bottom = (WORLD_H - 6).max(lava_line + 1);
    for col in 1..WORLD_W - 1 {
        let ceil = ceiling[col as usize].clamp(1, bottom);
        for ty in ceil..bottom {
            world.set_raw(col, ty, BlockId::STONE);
            world.set_wall_raw(col, ty, WallId::STONE);
        }
    }

    let margin = scale_w(50).max(4);
    if WORLD_W <= margin * 2 + 2 {
        return;
    }
    let main_caves = (WORLD_W / 80).max(2);
    for _ in 0..main_caves {
        let cx = rng.range(margin, WORLD_W - margin);
        let ceil = ceiling[cx as usize];
        let cy = (ceil + rng.range(scale_h(20), scale_h(80).max(scale_h(20) + 1))).min(WORLD_H - 8);
        let strength = rng.range(8, 20) as f64;
        let steps = rng.range(40, 200);
        tile_runner(world, rng, cx, cy, strength, steps, None, 0.0, 0.0);
    }
    let small_caves = (WORLD_W / 40).max(4);
    for _ in 0..small_caves {
        let cx = rng.range(margin, WORLD_W - margin);
        let ceil = ceiling[cx as usize];
        let cy = (ceil + rng.range(scale_h(10), scale_h(100).max(scale_h(10) + 1))).min(WORLD_H - 6);
        let strength = rng.range(4, 12) as f64;
        let steps = rng.range(10, 60);
        tile_runner(world, rng, cx, cy, strength, steps, None, 0.0, 0.0);
    }
    for col in margin..WORLD_W - margin {
        if !rng.chance(50) {
            continue;
        }
        let chimney_bottom = ceiling[col as usize];
        let chimney_top = (chimney_bottom - rng.range(scale_h(30), scale_h(80).max(scale_h(30) + 1))).max(1);
        let width = scale_h(rng.range(3, 7)).max(1);
        for ty in chimney_top..chimney_bottom {
            for dx in -(width / 2)..=(width / 2) {
                let tx = col + dx;
                if tx > 0 && tx < WORLD_W - 1 && ty > 0 && ty < WORLD_H {
                    world.set_raw(tx, ty, BlockId::AIR);
                }
            }
        }
    }
}

fn place_oceans(world: &mut World, rng: &mut Rng, surface_y: &[i32]) {
    let depth = floor_ratio(WORLD_H as f64 * 0.07);
    let width = floor_ratio(WORLD_W as f64 * 0.12);
    place_ocean(world, rng, -1, width, depth, surface_y);
    place_ocean(world, rng, 1, width, depth, surface_y);
}

/// `direction`：`-1` 左侧，`1` 右侧。沙底按离岸距离指数变深。深封层没有砂岩，用石头。
fn place_ocean(
    world: &mut World,
    rng: &mut Rng,
    direction: i32,
    ocean_width: i32,
    max_depth: i32,
    surface_y: &[i32],
) {
    if ocean_width < 2 || max_depth < 1 {
        return;
    }
    let (ocean_start, ocean_end, inland) = if direction < 0 {
        let end = ocean_width.min(WORLD_W - 1);
        (1, end, end.min(WORLD_W - 1))
    } else {
        let start = (WORLD_W - ocean_width).max(1);
        (start, WORLD_W - 1, start)
    };
    let span = (ocean_end - ocean_start).max(1) as f64;
    let sea_level = find_surface(world, inland, 1, WORLD_H / 2).unwrap_or_else(|| {
        surface_y
            .get(inland as usize)
            .copied()
            .unwrap_or(floor_ratio(WORLD_H as f64 * 0.3))
    });
    let mut sand_jitter = 0.0;

    for col in ocean_start..ocean_end {
        let dist = if direction < 0 {
            1.0 - (col - ocean_start) as f64 / span
        } else {
            (col - ocean_start) as f64 / span
        };
        let depth = max_depth as f64 * (1.0 - (-3.5 * dist).exp());
        sand_jitter += (rng.unit() - 0.5) * 3.0;
        sand_jitter = sand_jitter.clamp(-8.0, 8.0);
        let sand_floor = (sea_level + (depth + sand_jitter) as i32).min(WORLD_H - 20).max(sea_level);

        let mut solid_below = sand_floor;
        let scan_end = WORLD_H - 6;
        for ty in sand_floor..scan_end {
            if world.get(col, ty).solid() {
                solid_below = ty;
                break;
            }
            solid_below = ty;
        }
        let sand_bottom = (solid_below + 8).max(sand_floor + 15).min(WORLD_H - 6);

        for ty in sea_level..sand_floor {
            if world.get(col, ty).solid() {
                world.set_raw(col, ty, BlockId::AIR);
                world.clear_fluid(col, ty);
            }
        }
        for ty in sea_level..sand_floor {
            if !world.get(col, ty).solid() {
                world.set_raw(col, ty, BlockId::WATER);
                world.set_fluid(col, ty, FluidLevel::SOURCE);
            }
        }
        for ty in sand_floor..sand_bottom {
            let id = if ty < sand_floor + 8 {
                BlockId::SAND
            } else {
                BlockId::STONE
            };
            world.set_raw(col, ty, id);
        }
    }
}

fn place_snow(world: &mut World, rng: &mut Rng, base_surface: i32, base_rock: i32) {
    let x = floor_ratio(WORLD_W as f64 * 0.65);
    let w = floor_ratio(WORLD_W as f64 * 0.25);
    let y = base_surface - scale_h(10);
    let h = base_rock - base_surface + scale_h(30);
    // 石头转冰、泥转雪泥都没有对应方块，只转泥土和草。
    convert_columns(world, rng, x, y, w, h, Shape::Trapezoid, |id| match id {
        BlockId::DIRT | BlockId::GRASS => Some(BlockId::SNOW),
        _ => None,
    });
}

fn place_desert(world: &mut World, rng: &mut Rng, base_surface: i32, lava_line: i32) {
    let x = floor_ratio(WORLD_W as f64 * 0.42);
    let w = floor_ratio(WORLD_W as f64 * 0.10);
    let y = base_surface - scale_h(5);
    let h = lava_line - base_surface + scale_h(5);
    let surface_depth = 10.max(h / 4);
    convert_rect(world, rng, x, y, w, h, Shape::Ellipse, |id, ty| {
        if !matches!(id, BlockId::DIRT | BlockId::GRASS | BlockId::STONE | BlockId::SNOW) {
            return None;
        }
        // 没有硬化沙。表层和中层都是沙。再往下没有砂岩，用石头封。
        if ty < y + surface_depth * 3 {
            Some(BlockId::SAND)
        } else {
            Some(BlockId::STONE)
        }
    });
    let caves = 2.max((w * h) / 10000);
    for _ in 0..caves {
        let cx = x + rng.range(0, w.max(1));
        let cy = y + surface_depth + rng.range(0, (h - surface_depth).max(1));
        wander(world, rng, cx, cy, 4, 10, 10, 40, None, 0.0, 0.0);
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Trapezoid,
    Ellipse,
}

fn convert_columns(
    world: &mut World,
    rng: &mut Rng,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    shape: Shape,
    mut map_tile: impl FnMut(BlockId) -> Option<BlockId>,
) {
    if w < 2 || h < 2 {
        return;
    }
    let margin = 10;
    let x1 = (x - margin).max(1);
    let x2 = (x + w + margin).min(WORLD_W - 1);
    for col in x1..x2 {
        let surface = find_surface(world, col, (y - 30).max(1), y + h).unwrap_or(y);
        let bottom = (surface + h).min(WORLD_H - 1);
        for ty in surface..bottom {
            if !inside_shape(rng, col, ty, x, y, w, h, shape) {
                continue;
            }
            paint_if(world, col, ty, &mut map_tile);
        }
    }
}

fn convert_rect(
    world: &mut World,
    rng: &mut Rng,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    shape: Shape,
    mut map_tile: impl FnMut(BlockId, i32) -> Option<BlockId>,
) {
    if w < 2 || h < 2 {
        return;
    }
    let margin = 10;
    let x1 = (x - margin).max(1);
    let x2 = (x + w + margin).min(WORLD_W - 1);
    let y1 = y.max(1);
    let y2 = (y + h).min(WORLD_H - 1);
    for col in x1..x2 {
        for ty in y1..y2 {
            if !inside_shape(rng, col, ty, x, y, w, h, shape) {
                continue;
            }
            let cur = world.get(col, ty);
            if !cur.solid() || cur.frame_important() {
                continue;
            }
            if let Some(next) = map_tile(cur, ty) {
                if next != cur {
                    world.set_raw(col, ty, next);
                }
            }
        }
    }
}

fn paint_if(world: &mut World, col: i32, ty: i32, map_tile: &mut impl FnMut(BlockId) -> Option<BlockId>) {
    let cur = world.get(col, ty);
    if !cur.solid() || cur.frame_important() {
        return;
    }
    if let Some(next) = map_tile(cur) {
        if next != cur {
            world.set_raw(col, ty, next);
        }
    }
}

fn inside_shape(rng: &mut Rng, col: i32, row: i32, x: i32, y: i32, w: i32, h: i32, shape: Shape) -> bool {
    let center_x = x as f64 + w as f64 * 0.5;
    let half_w = (w as f64 * 0.5).max(1.0);
    let safe_h = h.max(1) as f64;
    let nx = (col as f64 - center_x) / half_w;
    let ny = (row - y) as f64 / safe_h;
    let jitter = (rng.unit() - 0.5) * 0.12;
    match shape {
        Shape::Ellipse => {
            let nyc = ny * 2.0 - 1.0;
            nx * nx + nyc * nyc <= 1.0 + jitter
        }
        Shape::Trapezoid => {
            let effective = 1.0 + 0.5 * ny;
            nx.abs() <= effective + jitter && (-0.05..=1.05).contains(&ny)
        }
    }
}

fn plant_trees(world: &mut World, rng: &mut Rng, surface_y: &[i32]) {
    let beach = floor_ratio(WORLD_W as f64 * 0.08);
    let spawn = WORLD_W / 2;
    let density = 0.15;
    let min_spacing = 4;
    let avg_spacing = (1.0 / density) as i32;
    let avg_spacing = avg_spacing.max(min_spacing);
    let mut x = beach;
    let end = WORLD_W - beach;
    while x < end {
        let near_spawn = (x - spawn).abs() < 8;
        let sy = surface_y.get(x as usize).copied().unwrap_or(0);
        let soil = world.get(x, sy);
        let above_clear = sy > 0 && world.get(x, sy - 1) == BlockId::AIR;
        if !near_spawn
            && above_clear
            && matches!(soil, BlockId::GRASS | BlockId::DIRT | BlockId::SNOW)
        {
            let trunk = rng.range(5, 17);
            world.grow_tree_at(x, sy, trunk);
        }
        let step = rng.range(min_spacing, avg_spacing + min_spacing / 2 + 1);
        x += step.max(1);
    }
}

fn find_surface(world: &World, x: i32, y0: i32, y1: i32) -> Option<i32> {
    if x <= 0 || x >= WORLD_W - 1 {
        return None;
    }
    let y0 = y0.max(0);
    let y1 = y1.min(WORLD_H - 1);
    for y in y0..=y1 {
        if world.get(x, y).solid() {
            return Some(y);
        }
    }
    None
}

fn wander(
    world: &mut World,
    rng: &mut Rng,
    x: i32,
    y: i32,
    s_lo: i32,
    s_hi: i32,
    t_lo: i32,
    t_hi: i32,
    paint: Option<BlockId>,
    speed_x: f64,
    speed_y: f64,
) {
    let strength = rng.range(s_lo, s_hi) as f64;
    let steps = rng.range(t_lo, t_hi);
    tile_runner(world, rng, x, y, strength, steps, paint, speed_x, speed_y);
}

/// 菱形漫游团块。`paint=None` 为挖空。速度为 0 表示当场随机。每步都加抖动，再钳到 `[-1, 1]`。
fn tile_runner(
    world: &mut World,
    rng: &mut Rng,
    x: i32,
    y: i32,
    strength: f64,
    steps: i32,
    paint: Option<BlockId>,
    speed_x: f64,
    speed_y: f64,
) {
    // 步数决定洞有多长，按当前高度相对小世界缩短。强度是洞的截面，按格计，不缩短。
    let steps = ((steps as f64) * (WORLD_H as f64 / REF_H)).round().max(1.0) as i32;
    if steps <= 0 || strength <= 0.0 {
        return;
    }
    let mut remaining = steps as f64;
    let total = remaining;
    let mut pos_x = x as f64;
    let mut pos_y = y as f64;
    let mut vel_x = if speed_x != 0.0 {
        speed_x
    } else {
        rng.range(-10, 11) as f64 * 0.1
    };
    let mut vel_y = if speed_y != 0.0 {
        speed_y
    } else {
        rng.range(-10, 11) as f64 * 0.1
    };

    while strength > 0.0 && remaining > 0.0 {
        let radius = strength * (remaining / total);
        remaining -= 1.0;

        let x0 = box_lo(pos_x - radius * 0.5);
        let x1 = box_hi(pos_x + radius * 0.5, WORLD_W - 1);
        let y0 = box_lo(pos_y - radius * 0.5);
        let y1 = box_hi(pos_y + radius * 0.5, WORLD_H - 1);

        for tx in x0..x1 {
            for ty in y0..y1 {
                let dist = (tx as f64 - pos_x).abs() + (ty as f64 - pos_y).abs();
                let threshold = strength * 0.5 * (1.0 + rng.range(-10, 11) as f64 * 0.015);
                if dist >= threshold {
                    continue;
                }
                match paint {
                    None => {
                        world.set_raw(tx, ty, BlockId::AIR);
                    }
                    Some(id) => {
                        if world.get(tx, ty).frame_important() {
                            continue;
                        }
                        world.set_raw(tx, ty, id);
                    }
                }
            }
        }

        pos_x += vel_x;
        pos_y += vel_y;
        vel_x += rng.range(-10, 11) as f64 * 0.05;
        vel_y += rng.range(-10, 11) as f64 * 0.05;
        vel_x = vel_x.clamp(-1.0, 1.0);
        vel_y = vel_y.clamp(-1.0, 1.0);
    }
}

fn box_lo(v: f64) -> i32 {
    let t = v as i32;
    if t < 1 { 1 } else { t }
}

fn box_hi(v: f64, limit: i32) -> i32 {
    let t = v as i32;
    if t > limit { limit } else { t }
}

