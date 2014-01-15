//! 地表生物群系（列分区，刻意保持薄）。

/// 地表群系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    /// 温带草甸（出生点附近）。
    Meadow,
    /// 密林。
    Forest,
    /// 荒原沙地。
    Desert,
    /// 寒地。
    Tundra,
}

impl BiomeId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Meadow => "草甸",
            Self::Forest => "密林",
            Self::Desert => "荒原",
            Self::Tundra => "寒地",
        }
    }

    /// 树生成权重（0..=100 近似概率）。
    pub fn tree_chance(self) -> u32 {
        match self {
            Self::Meadow => 28,
            Self::Forest => 55,
            Self::Desert => 4,
            Self::Tundra => 12,
        }
    }
}

/// 由种子与列 X 决定地表群系（有限边界分段）。
pub fn biome_at(seed: u64, x: i32) -> BiomeId {
    // 与 `tr-game` 世界宽 / 出生列对齐；夹具世界放大时同步改这里。
    const WORLD_W: i32 = 420;
    const SPAWN_X: i32 = WORLD_W / 2;
    let tx = x.clamp(0, WORLD_W - 1);
    // 出生点附近强制草甸，避免开局困在沙/雪
    if (tx - SPAWN_X).abs() < 28 {
        return BiomeId::Meadow;
    }
    let band = (tx * 4) / WORLD_W;
    let jitter = ((seed.wrapping_mul(0x9E37_79B9) ^ (tx as u64).wrapping_mul(17)) % 3) as i32;
    match (band + jitter).rem_euclid(4) {
        0 => BiomeId::Meadow,
        1 => BiomeId::Forest,
        2 => BiomeId::Desert,
        _ => BiomeId::Tundra,
    }
}
