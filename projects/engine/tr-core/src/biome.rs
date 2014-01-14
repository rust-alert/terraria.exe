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

/// 由种子与列 X 决定地表群系（绕环四段为主）。
pub fn biome_at(seed: u64, x: i32) -> BiomeId {
    let w = 160i32;
    let tx = ((x % w) + w) % w;
    // 出生点附近强制草甸，避免开局困在沙/雪
    let spawn = 24;
    let dist = (tx - spawn).rem_euclid(w).min((spawn - tx).rem_euclid(w));
    if dist < 18 {
        return BiomeId::Meadow;
    }
    let band = (tx * 4) / w;
    let jitter = ((seed.wrapping_mul(0x9E37_79B9) ^ (tx as u64).wrapping_mul(17)) % 3) as i32;
    match (band + jitter).rem_euclid(4) {
        0 => BiomeId::Meadow,
        1 => BiomeId::Forest,
        2 => BiomeId::Desert,
        _ => BiomeId::Tundra,
    }
}
