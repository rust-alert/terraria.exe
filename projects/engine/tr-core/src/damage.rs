//! 伤害类型与抗性矩阵（种族气质对齐）。

/// 伤害气质；与六族主伤害对应，环境伤害用 `True`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageType {
    /// 真实伤害：坠落等，无视抗性矩阵。
    True,
    /// 动能（人类主气质）。
    Kinetic,
    /// 元素（精灵）。
    Elemental,
    /// 热能（矮人）。
    Thermal,
    /// 能量（机械族）。
    Energy,
    /// 毒素 / DoT（共生体）。
    Toxin,
    /// 重力（晶核族）。
    Gravity,
}

impl DamageType {
    pub fn name(self) -> &'static str {
        match self {
            Self::True => "真实",
            Self::Kinetic => "动能",
            Self::Elemental => "元素",
            Self::Thermal => "热能",
            Self::Energy => "能量",
            Self::Toxin => "毒素",
            Self::Gravity => "重力",
        }
    }

    pub const COMBAT: [Self; 6] = [
        Self::Kinetic,
        Self::Elemental,
        Self::Thermal,
        Self::Energy,
        Self::Toxin,
        Self::Gravity,
    ];
}

/// 一次命中包。
#[derive(Debug, Clone, Copy)]
pub struct DamageHit {
    pub amount: f32,
    pub dtype: DamageType,
}

impl DamageHit {
    pub fn new(amount: f32, dtype: DamageType) -> Self {
        Self { amount, dtype }
    }

    pub fn kinetic(amount: f32) -> Self {
        Self::new(amount, DamageType::Kinetic)
    }

    pub fn environmental(amount: f32) -> Self {
        Self::new(amount, DamageType::True)
    }
}

/// 抗性档案：乘数 `1.0` 为中性，`<1` 抗性，`>1` 虚弱。
#[derive(Debug, Clone, Copy)]
pub struct ResistProfile {
    pub kinetic: f32,
    pub elemental: f32,
    pub thermal: f32,
    pub energy: f32,
    pub toxin: f32,
    pub gravity: f32,
}

impl Default for ResistProfile {
    fn default() -> Self {
        Self::neutral()
    }
}

impl ResistProfile {
    pub const fn neutral() -> Self {
        Self {
            kinetic: 1.0,
            elemental: 1.0,
            thermal: 1.0,
            energy: 1.0,
            toxin: 1.0,
            gravity: 1.0,
        }
    }

    /// 暗影凝胶：偏怕元素/热能，略抗动能与毒素。
    pub const fn slime() -> Self {
        Self {
            kinetic: 0.9,
            elemental: 1.35,
            thermal: 1.25,
            energy: 1.05,
            toxin: 0.55,
            gravity: 1.0,
        }
    }

    /// 木甲携带者：略抗动能。
    pub const fn wood_armor() -> Self {
        Self {
            kinetic: 0.85,
            elemental: 1.0,
            thermal: 1.05,
            energy: 1.0,
            toxin: 1.0,
            gravity: 1.0,
        }
    }

    pub fn multiplier(self, dtype: DamageType) -> f32 {
        match dtype {
            DamageType::True => 1.0,
            DamageType::Kinetic => self.kinetic,
            DamageType::Elemental => self.elemental,
            DamageType::Thermal => self.thermal,
            DamageType::Energy => self.energy,
            DamageType::Toxin => self.toxin,
            DamageType::Gravity => self.gravity,
        }
    }
}

/// 经抗性矩阵结算后的最终伤害（仍可为 0）。
pub fn resolve_damage(hit: DamageHit, resist: ResistProfile) -> f32 {
    if hit.amount <= 0.0 {
        return 0.0;
    }
    let mul = resist.multiplier(hit.dtype);
    (hit.amount * mul).max(0.0)
}
