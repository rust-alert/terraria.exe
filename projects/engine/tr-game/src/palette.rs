//! Terraria 场景与 HUD 统一调色板。
//!
//! 目标：避免 `world_view` / `tiles` / `hud` 各自硬编码 `Color::rgb`，
//! 让草土石水、天空冷暖与深色玻璃 HUD 共用同一套色温语言。

use spark_core::Color;

/// 材质色：基础 / 阴影 / 高光 / 边缘。
#[derive(Debug, Clone, Copy)]
pub struct MaterialSwatch {
    pub base: Color,
    pub shade: Color,
    pub highlight: Color,
    pub rim: Color,
}

impl MaterialSwatch {
    pub const fn new(base: Color, shade: Color, highlight: Color, rim: Color) -> Self {
        Self {
            base,
            shade,
            highlight,
            rim,
        }
    }
}

/// 土壤：暖棕、粗糙、几乎无高光。
pub const DIRT: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.48, 0.30, 0.17),
    Color::rgb(0.28, 0.16, 0.09),
    Color::rgb(0.58, 0.40, 0.24),
    Color::rgb(0.22, 0.12, 0.07),
);

/// 草地：顶面鲜绿，边缘偏冷。
pub const GRASS: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.30, 0.62, 0.28),
    Color::rgb(0.14, 0.36, 0.16),
    Color::rgb(0.48, 0.78, 0.36),
    Color::rgb(0.20, 0.48, 0.22),
);

/// 草→土过渡层。
pub const GRASS_SOIL: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.40, 0.48, 0.20),
    Color::rgb(0.30, 0.34, 0.14),
    Color::rgb(0.52, 0.58, 0.28),
    Color::rgb(0.26, 0.30, 0.12),
);

/// 石头：冷灰，低强度高光。
pub const STONE: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.44, 0.46, 0.52),
    Color::rgb(0.26, 0.28, 0.32),
    Color::rgb(0.62, 0.66, 0.72),
    Color::rgb(0.18, 0.20, 0.24),
);

/// 水面：冷蓝，偏透明。
pub const WATER: MaterialSwatch = MaterialSwatch::new(
    Color::rgba(0.16, 0.42, 0.78, 0.72),
    Color::rgba(0.08, 0.24, 0.48, 0.80),
    Color::rgba(0.55, 0.82, 0.98, 0.55),
    Color::rgba(0.12, 0.32, 0.62, 0.55),
);

/// 沙：暖黄，顶面亮、底边暗。
pub const SAND: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.84, 0.72, 0.42),
    Color::rgb(0.58, 0.44, 0.22),
    Color::rgb(0.96, 0.88, 0.62),
    Color::rgb(0.46, 0.34, 0.16),
);

/// 雪：冷白，阴影偏蓝。
pub const SNOW: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.88, 0.92, 0.96),
    Color::rgb(0.58, 0.66, 0.78),
    Color::rgb(0.98, 0.99, 1.0),
    Color::rgb(0.42, 0.50, 0.62),
);

/// 铜矿脉：暖橙，嵌在石面上。
pub const COPPER: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.78, 0.42, 0.18),
    Color::rgb(0.42, 0.20, 0.08),
    Color::rgb(0.98, 0.68, 0.32),
    Color::rgb(0.28, 0.12, 0.06),
);

/// 铁矿脉：冷银，嵌在石面上。
pub const IRON: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.70, 0.74, 0.80),
    Color::rgb(0.36, 0.40, 0.46),
    Color::rgb(0.92, 0.94, 0.98),
    Color::rgb(0.22, 0.24, 0.28),
);

/// 木材：暖棕，竖向年轮。
pub const WOOD: MaterialSwatch = MaterialSwatch::new(
    Color::rgb(0.58, 0.36, 0.18),
    Color::rgb(0.34, 0.18, 0.08),
    Color::rgb(0.78, 0.55, 0.30),
    Color::rgb(0.22, 0.12, 0.06),
);

/// 树叶：鲜绿，半透明空洞。
pub const LEAF: MaterialSwatch = MaterialSwatch::new(
    Color::rgba(0.28, 0.62, 0.28, 0.90),
    Color::rgba(0.12, 0.36, 0.14, 0.92),
    Color::rgba(0.48, 0.82, 0.36, 0.85),
    Color::rgba(0.16, 0.42, 0.18, 0.88),
);

/// 天空色带（日间顶 / 日间底 / 黄昏中层）。
#[derive(Debug, Clone, Copy)]
pub struct SkyBand {
    pub day_top: Color,
    pub day_bot: Color,
    pub dusk: Color,
    pub ridge: (f32, f32, f32),
    pub hill: (f32, f32, f32),
}

pub const SKY_MEADOW: SkyBand = SkyBand {
    day_top: Color::rgb(0.14, 0.28, 0.55),
    day_bot: Color::rgb(0.78, 0.86, 0.96),
    dusk: Color::rgb(0.62, 0.34, 0.48),
    ridge: (0.06, 0.09, 0.10),
    hill: (0.05, 0.10, 0.08),
};

pub const SKY_FOREST: SkyBand = SkyBand {
    day_top: Color::rgb(0.10, 0.24, 0.40),
    day_bot: Color::rgb(0.58, 0.74, 0.80),
    dusk: Color::rgb(0.42, 0.28, 0.40),
    ridge: (0.04, 0.09, 0.07),
    hill: (0.03, 0.08, 0.05),
};

pub const SKY_DESERT: SkyBand = SkyBand {
    day_top: Color::rgb(0.32, 0.42, 0.68),
    day_bot: Color::rgb(0.94, 0.80, 0.58),
    dusk: Color::rgb(0.80, 0.42, 0.36),
    ridge: (0.16, 0.11, 0.08),
    hill: (0.20, 0.14, 0.08),
};

pub const SKY_TUNDRA: SkyBand = SkyBand {
    day_top: Color::rgb(0.18, 0.26, 0.46),
    day_bot: Color::rgb(0.82, 0.88, 0.96),
    dusk: Color::rgb(0.48, 0.36, 0.58),
    ridge: (0.09, 0.11, 0.15),
    hill: (0.11, 0.13, 0.17),
};

pub const SKY_NIGHT_TOP: Color = Color::rgb(0.02, 0.03, 0.08);
pub const SKY_NIGHT_BOT: Color = Color::rgb(0.08, 0.07, 0.14);
pub const SKY_DAY_MID: Color = Color::rgb(0.42, 0.52, 0.78);

/// HUD：深色玻璃。
pub const HUD_PANEL: Color = Color::rgba(0.04, 0.07, 0.12, 0.62);
pub const HUD_PANEL_EDGE: Color = Color::rgba(0.42, 0.68, 0.95, 0.42);
pub const HUD_TEXT_MAIN: Color = Color::rgb(0.94, 0.96, 1.0);
pub const HUD_TEXT_DIM: Color = Color::rgb(0.58, 0.68, 0.80);
pub const HUD_ACCENT: Color = Color::rgb(0.55, 0.86, 1.0);
pub const HUD_HOTBAR_IDLE: Color = Color::rgba(0.05, 0.07, 0.10, 0.72);
pub const HUD_HOTBAR_SELECTED: Color = Color::rgb(0.14, 0.28, 0.42);
pub const HUD_HOTBAR_RING: Color = Color::rgb(0.72, 0.92, 1.0);
pub const HUD_HOTBAR_RING_IDLE: Color = Color::rgb(0.20, 0.24, 0.32);

/// 光源染色（彩色光照 P1 前先统一常量，避免各处硬编码）。
pub const GLOW_TORCH: Color = Color::rgb(1.0, 0.72, 0.32);
pub const GLOW_FURNACE: Color = Color::rgb(1.0, 0.45, 0.18);
pub const GLOW_POD: Color = Color::rgb(0.45, 0.85, 1.0);
pub const GLOW_WARP: Color = Color::rgb(0.78, 0.45, 1.0);
pub const GLOW_DEFAULT: Color = Color::rgb(1.0, 0.85, 0.55);
pub const GLOW_COPPER: Color = Color::rgb(1.0, 0.55, 0.22);
pub const GLOW_IRON: Color = Color::rgb(0.78, 0.86, 0.95);

/// 线性插值（用于噪声与昼夜）。
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}
