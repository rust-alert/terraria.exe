//! 权威 Tile 状态（世界存档 / framing / 渲染的共同前置）。
//!
//! 渲染器与操作层只能读写这些字段，不得在绘制阶段猜测玩法状态。

use crate::{BlockId, WallId};

/// 液体种类（语义对齐；内容覆盖随完成面扩展）。
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

/// 斜坡朝向（与 slope 枚举对齐的占位；具体数值以 `.wld` 样本为准）。
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

    /// 开发会话 / 测试用的固定字节宽度。不是 `.wld` 列编码。
    pub const WIRE_LEN: usize = 15;

    /// 写出固定宽度字节。字段顺序与 [`Tile::from_wire`] 对应。
    pub fn to_wire(self) -> [u8; Self::WIRE_LEN] {
        let mut out = [0u8; Self::WIRE_LEN];
        out[0..4].copy_from_slice(&self.tile_type.0.to_le_bytes());
        out[4] = self.wall_type.0;
        out[5] = self.liquid_amount;
        out[6] = self.liquid_kind as u8;
        out[7] = self.slope as u8;
        out[8..10].copy_from_slice(&self.frame_x.to_le_bytes());
        out[10..12].copy_from_slice(&self.frame_y.to_le_bytes());
        let mut flags = 0u8;
        if self.half_block {
            flags |= 1 << 0;
        }
        if self.wire_red {
            flags |= 1 << 1;
        }
        if self.wire_blue {
            flags |= 1 << 2;
        }
        if self.wire_green {
            flags |= 1 << 3;
        }
        if self.wire_yellow {
            flags |= 1 << 4;
        }
        if self.actuator {
            flags |= 1 << 5;
        }
        if self.inactive {
            flags |= 1 << 6;
        }
        out[12] = flags;
        out[13] = self.paint;
        out[14] = self.wall_paint;
        out
    }

    /// 从 [`Tile::to_wire`] 字节还原。长度不对或枚举越界则失败。
    pub fn from_wire(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != Self::WIRE_LEN {
            return Err("tile wire length");
        }
        let tile_type = BlockId(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]));
        let wall_type = WallId(bytes[4]);
        let liquid_amount = bytes[5];
        let liquid_kind = match bytes[6] {
            0 => LiquidKind::None,
            1 => LiquidKind::Water,
            2 => LiquidKind::Lava,
            3 => LiquidKind::Honey,
            4 => LiquidKind::Shimmer,
            _ => return Err("liquid kind"),
        };
        let slope = match bytes[7] {
            0 => SlopeKind::None,
            1 => SlopeKind::BottomRight,
            2 => SlopeKind::BottomLeft,
            3 => SlopeKind::TopRight,
            4 => SlopeKind::TopLeft,
            _ => return Err("slope"),
        };
        let frame_x = i16::from_le_bytes([bytes[8], bytes[9]]);
        let frame_y = i16::from_le_bytes([bytes[10], bytes[11]]);
        let flags = bytes[12];
        Ok(Self {
            tile_type,
            wall_type,
            frame_x,
            frame_y,
            liquid_amount,
            liquid_kind,
            slope,
            half_block: flags & (1 << 0) != 0,
            wire_red: flags & (1 << 1) != 0,
            wire_blue: flags & (1 << 2) != 0,
            wire_green: flags & (1 << 3) != 0,
            wire_yellow: flags & (1 << 4) != 0,
            actuator: flags & (1 << 5) != 0,
            inactive: flags & (1 << 6) != 0,
            paint: bytes[13],
            wall_paint: bytes[14],
        })
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

    #[test]
    fn wire_bytes_roundtrip_keeps_flags_and_frames() {
        let mut t = Tile::solid(BlockId::DIRT);
        t.wall_type = WallId::WOOD;
        t.set_frame(36, 18);
        t.set_liquid(LiquidKind::Water, 200);
        t.half_block = true;
        t.wire_red = true;
        t.wire_yellow = true;
        t.actuator = true;
        t.paint = 3;
        t.wall_paint = 5;
        let bytes = t.to_wire();
        assert_eq!(bytes.len(), Tile::WIRE_LEN);
        let back = Tile::from_wire(&bytes).unwrap();
        assert_eq!(back, t);
        assert!(Tile::from_wire(&bytes[..14]).is_err());
        let mut bad = bytes;
        bad[6] = 9;
        assert!(Tile::from_wire(&bad).is_err());
    }
}
