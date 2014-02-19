//! 液体元数据（过渡）。
//!
//! **现状（未经验证）**：仍沿用 `0..=8` 水位与源水语义，**不是** `0..=255`
//! 液量模型。权威路径建立前禁止继续扩展该语义；运行时流体步进已冻结。
//!
//! 目标：改为 `liquid_amount: u8`（`0..=255`）+ `liquid_kind`，由世界事务更新。

/// 过渡期水位（仅 [`crate::BlockId::WATER`]；待替换为完整液量）。
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

    /// 格内填充比例 `0..=1`（自格底向上）。过渡近似，非正式曲面。
    pub fn fill_ratio(self) -> f32 {
        let lv = self.clamp_valid().0;
        if lv == 0 || lv >= 8 {
            1.0
        } else {
            (8 - lv) as f32 / 8.0
        }
    }

    /// 水平扩散下一格水位。**过渡期 API**，权威液体实现后删除。
    pub fn spread_next(self) -> Option<Self> {
        let base = if self.0 == 0 || self.0 >= 8 {
            1
        } else {
            self.0.saturating_add(1)
        };
        if base > 7 { None } else { Some(Self(base)) }
    }

    /// 变浅一档。**过渡期 API**，权威液体实现后删除。
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
