//! 权威 Tile 状态（世界存档 / framing / 渲染的共同前置）。
//!
//! 渲染器与操作层只能读写这些字段，不得在绘制阶段猜测玩法状态。

use crate::{BlockId, WallId};

/// 液体种类（正版语义对齐；内容覆盖随完成面扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LiquidKind {
    #[default]
    None = 0,
    Water = 1,
    Lava = 2,
    Honey = 3,
    Shimmer = 4,
}

/// 斜坡朝向（与正版 slope 枚举对齐的占位；具体数值以 `.wld` 样本为准）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SlopeKind {
    #[default]
    None = 0,
    BottomRight = 1,
    BottomLeft = 2,
    TopRight = 3,
    TopLeft = 4,
}

/// 单格权威状态。紧凑布局，供分区世界与修改事务使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    /// 前景物块类型；`BlockId::AIR` 表示空。
    pub tile_type: BlockId,
    /// 背景墙类型；`WallId::NONE` 表示无墙。
    pub wall_type: WallId,
    /// 图集帧 X（像素或步长单位，由 framing 写入）。
    pub frame_x: i16,
    /// 图集帧 Y。
    pub frame_y: i16,
    /// 液量 `0..=255`；`0` 表示无液体。
    pub liquid_amount: u8,
    /// 液体种类。
    pub liquid_kind: LiquidKind,
    /// 斜坡。
    pub slope: SlopeKind,
    /// 半砖。
    pub half_block: bool,
    pub wire_red: bool,
    pub wire_blue: bool,
    pub wire_green: bool,
    pub wire_yellow: bool,
    pub actuator: bool,
    /// 执行器关闭后的非碰撞态。
    pub inactive: bool,
    /// 物块涂料（0 = 无）。
    pub paint: u8,
    /// 墙涂料（0 = 无）。
    pub wall_paint: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self::empty()
    }
}

impl Tile {
    /// 空空气格。
    pub const fn empty() -> Self {
        Self {
            tile_type: BlockId::AIR,
            wall_type: WallId::NONE,
            frame_x: 0,
            frame_y: 0,
            liquid_amount: 0,
            liquid_kind: LiquidKind::None,
            slope: SlopeKind::None,
            half_block: false,
            wire_red: false,
            wire_blue: false,
            wire_green: false,
            wire_yellow: false,
            actuator: false,
            inactive: false,
            paint: 0,
            wall_paint: 0,
        }
    }

    /// 实心物块（无墙、无液、默认帧）。
    pub const fn solid(block: BlockId) -> Self {
        let mut t = Self::empty();
        t.tile_type = block;
        t
    }

    /// 是否有前景物块。
    pub fn has_tile(self) -> bool {
        self.tile_type != BlockId::AIR
    }

    /// 是否有背景墙。
    pub fn has_wall(self) -> bool {
        self.wall_type != WallId::NONE
    }

    /// 是否有液体。
    pub fn has_liquid(self) -> bool {
        self.liquid_amount > 0 && self.liquid_kind != LiquidKind::None
    }

    /// 写入 framing 结果。
    pub fn set_frame(&mut self, frame_x: i16, frame_y: i16) {
        self.frame_x = frame_x;
        self.frame_y = frame_y;
    }

    /// 写入液量；`amount == 0` 时清种类。
    pub fn set_liquid(&mut self, kind: LiquidKind, amount: u8) {
        if amount == 0 || kind == LiquidKind::None {
            self.liquid_amount = 0;
            self.liquid_kind = LiquidKind::None;
        } else {
            self.liquid_kind = kind;
            self.liquid_amount = amount;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tile_has_no_content() {
        let t = Tile::empty();
        assert!(!t.has_tile());
        assert!(!t.has_wall());
        assert!(!t.has_liquid());
        assert_eq!(t.frame_x, 0);
        assert_eq!(t.frame_y, 0);
    }

    #[test]
    fn solid_and_frame_roundtrip() {
        let mut t = Tile::solid(BlockId::DIRT);
        assert!(t.has_tile());
        t.set_frame(18, 18);
        assert_eq!((t.frame_x, t.frame_y), (18, 18));
    }

    #[test]
    fn liquid_clears_when_empty() {
        let mut t = Tile::empty();
        t.set_liquid(LiquidKind::Water, 255);
        assert!(t.has_liquid());
        t.set_liquid(LiquidKind::Water, 0);
        assert!(!t.has_liquid());
        assert_eq!(t.liquid_kind, LiquidKind::None);
    }

    #[test]
    fn tile_is_copy_and_compact() {
        let a = Tile::solid(BlockId::STONE);
        let b = a;
        assert_eq!(a, b);
        // 紧凑布局：不应接近旧式多 Vec 分表的单格开销。
        assert!(std::mem::size_of::<Tile>() <= 32);
    }
}
