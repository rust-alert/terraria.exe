//! Minecraft 风格流体元数据（水位 `0..=8`）。
//!
//! - `0`：源格（满格）
//! - `1..=7`：水平流动，数值越大越浅
//! - `8`：下落中（视觉满格）

/// 流体水位（与方块分存；仅 [`crate::BlockId::WATER`] 有效）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FluidLevel(pub u8);

impl FluidLevel {
    pub const SOURCE: Self = Self(0);
    pub const FALLING: Self = Self(8);

    pub fn clamp_valid(self) -> Self {
        Self(self.0.min(8))
    }

    pub fn is_source(self) -> bool {
        self.0 == 0
    }

    pub fn is_falling(self) -> bool {
        self.0 >= 8
    }

    /// 格内填充比例 `0..=1`（自格底向上）。
    pub fn fill_ratio(self) -> f32 {
        let lv = self.clamp_valid().0;
        if lv == 0 || lv >= 8 {
            1.0
        } else {
            (8 - lv) as f32 / 8.0
        }
    }

    /// 水平扩散下一格水位；源/下落按 1 起步。已到最浅则 `None`。
    pub fn spread_next(self) -> Option<Self> {
        let base = if self.0 == 0 || self.0 >= 8 {
            1
        } else {
            self.0.saturating_add(1)
        };
        if base > 7 { None } else { Some(Self(base)) }
    }

    /// 变浅一档；已是最浅或下落则清空（返回 `None` 表示应移除）。
    pub fn recede(self) -> Option<Self> {
        if self.is_source() {
            return Some(self);
        }
        if self.is_falling() {
            return None;
        }
        let n = self.0.saturating_add(1);
        if n > 7 { None } else { Some(Self(n)) }
    }
}
