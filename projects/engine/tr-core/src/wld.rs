//! `.wld` 只读解析：文件头、世界属性前缀、tile 段。
//!
//! 支持桌面世界文件版本 `140` 起的文件头。世界属性前缀要求 `315`（1.4.5.0）及以上。
//! tile 段要求 `269`（1.4.4）及以上。属性段在猩红标记之后的击杀计数与清单尚未读取，
//! tile 段通过文件头里的分区偏移定位，不依赖把属性段读完。

use std::fmt;

use crate::{LiquidKind, SlopeKind};

/// 1.4.5.0 起的世界文件版本。
pub const WORLD_VERSION_1_4_5: u32 = 315;
/// 1.4.5.8 的世界文件版本。
pub const WORLD_VERSION_1_4_5_8: u32 = 326;

const MAGIC_DESKTOP: &[u8; 7] = b"relogic";
const MAGIC_ALT: &[u8; 7] = b"xindong";
const FILE_TYPE_WORLD: u8 = 2;

/// `.wld` 读取失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WldError {
    /// 字节不足。
    Truncated,
    /// 魔数不是已知世界容器。
    BadMagic,
    /// 文件类型不是世界。
    BadFileType(u8),
    /// 版本低于本解析器支持范围。
    UnsupportedVersion(u32),
    /// 字符串不是 UTF-8。
    BadString,
    /// 分区表缺失或偏移非法。
    BadSection,
    /// RLE 超出当前列。
    RleOverflow,
    /// 斜坡字节不在已知范围。
    BadSlope(u8),
}

impl fmt::Display for WldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "世界文件被截断"),
            Self::BadMagic => write!(f, "世界文件魔数无效"),
            Self::BadFileType(t) => write!(f, "不是世界文件（类型 {t}）"),
            Self::UnsupportedVersion(v) => write!(f, "不支持的世界版本 {v}"),
            Self::BadString => write!(f, "世界字符串不是 UTF-8"),
            Self::BadSection => write!(f, "世界分区表无效"),
            Self::RleOverflow => write!(f, "物块游程超出列高"),
            Self::BadSlope(v) => write!(f, "未知斜坡字节 {v}"),
        }
    }
}

impl std::error::Error for WldError {}

/// 文件头：版本、分区偏移、frame-important 位图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WldFileHeader {
    /// 世界文件版本。
    pub version: u32,
    /// 7 字节容器魔数。
    pub magic: [u8; 7],
    /// 文件修订号。
    pub file_revision: u32,
    /// 收藏标记。
    pub favorite: bool,
    /// 各分区在文件中的字节偏移。
    pub section_pointers: Vec<i32>,
    /// 下标为物块类型，真表示该类型带 `frameX` / `frameY`。
    pub frame_important: Vec<bool>,
}

impl WldFileHeader {
    /// tile 段起始偏移。分区表第 2 项（下标 1）。
    pub fn tile_section_offset(&self) -> Result<usize, WldError> {
        let off = self
            .section_pointers
            .get(1)
            .copied()
            .ok_or(WldError::BadSection)?;
        if off < 0 {
            return Err(WldError::BadSection);
        }
        Ok(off as usize)
    }
}

/// 1.4.5 种子开关（属性前缀内）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WldSeedFlags {
    /// 醉酒世界。
    pub drunk: bool,
    /// For the Worthy。
    pub for_the_worthy: bool,
    /// 十周年。
    pub tenth_anniversary: bool,
    /// Don't Starve。
    pub dont_starve: bool,
    /// Not the Bees。
    pub not_the_bees: bool,
    /// Remix。
    pub remix: bool,
    /// 无陷阱。
    pub no_traps: bool,
    /// Zenith。
    pub zenith: bool,
    /// 空岛。
    pub skyblock: bool,
}

/// 世界属性前缀。猩红标记之后的字段本轮不读。
#[derive(Debug, Clone, PartialEq)]
pub struct WldProperties {
    /// 世界名。
    pub title: String,
    /// 种子文本。
    pub seed: String,
    /// 生成器版本号。
    pub world_gen_version: u64,
    /// 世界整数编号。
    pub world_id: i32,
    /// 左边界（像素）。
    pub left_world: i32,
    /// 右边界（像素）。
    pub right_world: i32,
    /// 上边界（像素）。
    pub top_world: i32,
    /// 下边界（像素）。
    pub bottom_world: i32,
    /// 格高（先写入的那个尺寸）。
    pub tiles_high: i32,
    /// 格宽。tile 段按宽为列、高为行展开。
    pub tiles_wide: i32,
    /// 游戏模式。
    pub game_mode: i32,
    /// 种子开关。
    pub seeds: WldSeedFlags,
    /// 出生列。
    pub spawn_x: i32,
    /// 出生行。
    pub spawn_y: i32,
    /// 地表层高。
    pub ground_level: f64,
    /// 岩石层高。
    pub rock_level: f64,
    /// 是否猩红（否则为腐化侧）。
    pub crimson: bool,
}

/// 从 `.wld` tile 段解出的一格。类型号是文件内编号，尚未映射到玩法 `BlockId`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WldCell {
    /// 前景类型。`None` 表示该格未激活。
    pub tile_type: Option<u16>,
    /// 帧 X。
    pub frame_x: i16,
    /// 帧 Y。
    pub frame_y: i16,
    /// 墙类型。`0` 表示无墙。
    pub wall: u16,
    /// 液量 `0..=255`。
    pub liquid_amount: u8,
    /// 液体种类。
    pub liquid_kind: LiquidKind,
    /// 斜坡。
    pub slope: SlopeKind,
    /// 半砖。
    pub half_block: bool,
    /// 红电线。
    pub wire_red: bool,
    /// 蓝电线。
    pub wire_blue: bool,
    /// 绿电线。
    pub wire_green: bool,
    /// 黄电线。
    pub wire_yellow: bool,
    /// 执行器。
    pub actuator: bool,
    /// 被执行器关闭。
    pub inactive: bool,
    /// 物块涂料。
    pub paint: u8,
    /// 墙涂料。
    pub wall_paint: u8,
}

impl Default for WldCell {
    fn default() -> Self {
        Self {
            tile_type: None,
            frame_x: 0,
            frame_y: 0,
            wall: 0,
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
}

struct Rd<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Rd<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }

    fn rest(&self) -> usize {
        self.b.len().saturating_sub(self.i)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WldError> {
        let end = self.i.checked_add(n).ok_or(WldError::Truncated)?;
        if end > self.b.len() {
            return Err(WldError::Truncated);
        }
        let s = &self.b[self.i..end];
        self.i = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, WldError> {
        Ok(self.take(1)?[0])
    }

    fn bool(&mut self) -> Result<bool, WldError> {
        Ok(self.u8()? != 0)
    }

    fn u16(&mut self) -> Result<u16, WldError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn i16(&mut self) -> Result<i16, WldError> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, WldError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32, WldError> {
        Ok(self.u32()? as i32)
    }

    fn u64(&mut self) -> Result<u64, WldError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, WldError> {
        Ok(self.u64()? as i64)
    }

    fn f64(&mut self) -> Result<f64, WldError> {
        Ok(f64::from_le_bytes(self.u64()?.to_le_bytes()))
    }

    fn net_i32(&mut self) -> Result<i32, WldError> {
        let mut value = 0i32;
        let mut shift = 0;
        loop {
            if shift > 28 {
                return Err(WldError::BadString);
            }
            let b = self.u8()?;
            value |= i32::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    fn net_string(&mut self) -> Result<String, WldError> {
        let n = self.net_i32()?;
        if n < 0 {
            return Err(WldError::BadString);
        }
        let bytes = self.take(n as usize)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WldError::BadString)
    }
}

fn read_bit_flags(r: &mut Rd<'_>) -> Result<Vec<bool>, WldError> {
    let len = r.i16()?;
    if len < 0 {
        return Err(WldError::BadSection);
    }
    let mut out = vec![false; len as usize];
    let mut data = 0u8;
    let mut mask = 128u8;
    for flag in &mut out {
        if mask == 128 {
            data = r.u8()?;
            mask = 1;
        } else {
            mask <<= 1;
        }
        *flag = data & mask == mask;
    }
    Ok(out)
}

/// 读取文件头。返回头和已消费字节数。版本低于 `140` 拒绝。
pub fn read_file_header(bytes: &[u8]) -> Result<(WldFileHeader, usize), WldError> {
    let mut r = Rd::new(bytes);
    let version = r.u32()?;
    if version < 140 {
        return Err(WldError::UnsupportedVersion(version));
    }
    let magic_b = r.take(7)?;
    let mut magic = [0u8; 7];
    magic.copy_from_slice(magic_b);
    if &magic != MAGIC_DESKTOP && &magic != MAGIC_ALT {
        return Err(WldError::BadMagic);
    }
    let file_type = r.u8()?;
    if file_type != FILE_TYPE_WORLD {
        return Err(WldError::BadFileType(file_type));
    }
    let file_revision = r.u32()?;
    let flags = r.u64()?;
    let section_count = r.i16()?;
    if section_count < 2 {
        return Err(WldError::BadSection);
    }
    let mut section_pointers = Vec::with_capacity(section_count as usize);
    for _ in 0..section_count {
        section_pointers.push(r.i32()?);
    }
    let frame_important = read_bit_flags(&mut r)?;
    Ok((
        WldFileHeader {
            version,
            magic,
            file_revision,
            favorite: flags & 1 == 1,
            section_pointers,
            frame_important,
        },
        r.i,
    ))
}

/// 读取 1.4.5 世界属性前缀（到猩红标记为止）。
pub fn read_world_properties(
    bytes: &[u8],
    version: u32,
) -> Result<(WldProperties, usize), WldError> {
    if version < WORLD_VERSION_1_4_5 {
        return Err(WldError::UnsupportedVersion(version));
    }
    let mut r = Rd::new(bytes);
    let title = r.net_string()?;
    let seed = r.net_string()?;
    let world_gen_version = r.u64()?;
    let _guid = r.take(16)?;
    let world_id = r.i32()?;
    let left_world = r.i32()?;
    let right_world = r.i32()?;
    let top_world = r.i32()?;
    let bottom_world = r.i32()?;
    let tiles_high = r.i32()?;
    let tiles_wide = r.i32()?;
    let game_mode = r.i32()?;
    let seeds = WldSeedFlags {
        drunk: r.bool()?,
        for_the_worthy: r.bool()?,
        tenth_anniversary: r.bool()?,
        dont_starve: r.bool()?,
        not_the_bees: r.bool()?,
        remix: r.bool()?,
        no_traps: r.bool()?,
        zenith: r.bool()?,
        skyblock: r.bool()?,
    };
    let _creation = r.i64()?;
    let _last_played = r.i64()?;
    let _moon = r.u8()?;
    let mut skip_i32 = |n: usize, r: &mut Rd<'_>| -> Result<(), WldError> {
        for _ in 0..n {
            r.i32()?;
        }
        Ok(())
    };
    skip_i32(3, &mut r)?;
    skip_i32(4, &mut r)?;
    skip_i32(3, &mut r)?;
    skip_i32(4, &mut r)?;
    skip_i32(3, &mut r)?;
    let spawn_x = r.i32()?;
    let spawn_y = r.i32()?;
    let ground_level = r.f64()?;
    let rock_level = r.f64()?;
    let _time = r.f64()?;
    let _day = r.bool()?;
    let _moon_phase = r.i32()?;
    let _blood = r.bool()?;
    let _eclipse = r.bool()?;
    let _dungeon_x = r.i32()?;
    let _dungeon_y = r.i32()?;
    let crimson = r.bool()?;
    Ok((
        WldProperties {
            title,
            seed,
            world_gen_version,
            world_id,
            left_world,
            right_world,
            top_world,
            bottom_world,
            tiles_high,
            tiles_wide,
            game_mode,
            seeds,
            spawn_x,
            spawn_y,
            ground_level,
            rock_level,
            crimson,
        },
        r.i,
    ))
}

fn slope_from_brick(style: u8) -> Result<(SlopeKind, bool), WldError> {
    match style {
        0 => Ok((SlopeKind::None, false)),
        1 => Ok((SlopeKind::None, true)),
        2 => Ok((SlopeKind::TopRight, false)),
        3 => Ok((SlopeKind::TopLeft, false)),
        4 => Ok((SlopeKind::BottomRight, false)),
        5 => Ok((SlopeKind::BottomLeft, false)),
        other => Err(WldError::BadSlope(other)),
    }
}

fn read_one_cell(
    r: &mut Rd<'_>,
    version: u32,
    frame_important: &[bool],
) -> Result<(WldCell, u16), WldError> {
    let header1 = r.u8()?;
    let header2 = if header1 & 0b0000_0001 != 0 {
        r.u8()?
    } else {
        0
    };
    let header3 = if header2 & 0b0000_0001 != 0 {
        r.u8()?
    } else {
        0
    };
    let header4 = if version >= 269 && header3 & 0b0000_0001 != 0 {
        r.u8()?
    } else {
        0
    };

    let mut cell = WldCell::default();
    if header1 & 0b0000_0010 != 0 {
        let tile_type = if header1 & 0b0010_0000 == 0 {
            u16::from(r.u8()?)
        } else {
            r.u16()?
        };
        cell.tile_type = Some(tile_type);
        let framed = frame_important
            .get(tile_type as usize)
            .copied()
            .unwrap_or(true);
        if framed {
            cell.frame_x = r.i16()?;
            cell.frame_y = r.i16()?;
        }
        if header3 & 0b0000_1000 != 0 {
            cell.paint = r.u8()?;
        }
    }

    if header1 & 0b0000_0100 != 0 {
        cell.wall = u16::from(r.u8()?);
        if header3 & 0b0001_0000 != 0 {
            cell.wall_paint = r.u8()?;
        }
    }

    let liquid_bits = (header1 & 0b0001_1000) >> 3;
    if liquid_bits != 0 {
        cell.liquid_amount = r.u8()?;
        cell.liquid_kind = match liquid_bits {
            1 => LiquidKind::Water,
            2 => LiquidKind::Lava,
            3 => LiquidKind::Honey,
            _ => LiquidKind::None,
        };
        if version >= 269 && header3 & 0b1000_0000 != 0 {
            cell.liquid_kind = LiquidKind::Shimmer;
        }
    }

    if header2 > 1 {
        cell.wire_red = header2 & 0b0000_0010 != 0;
        cell.wire_blue = header2 & 0b0000_0100 != 0;
        cell.wire_green = header2 & 0b0000_1000 != 0;
        let brick = (header2 & 0b0111_0000) >> 4;
        let (slope, half) = slope_from_brick(brick)?;
        cell.slope = slope;
        cell.half_block = half;
    }

    if header3 > 1 {
        cell.actuator = header3 & 0b0000_0010 != 0;
        cell.inactive = header3 & 0b0000_0100 != 0;
        cell.wire_yellow = header3 & 0b0010_0000 != 0;
        if version >= 222 && header3 & 0b0100_0000 != 0 {
            let hi = u16::from(r.u8()?);
            cell.wall |= hi << 8;
        }
    }

    let rle_kind = (header1 & 0b1100_0000) >> 6;
    let rle = match rle_kind {
        0 => 0u16,
        1 => u16::from(r.u8()?),
        _ => r.u16()?,
    };
    let _ = header4;
    Ok((cell, rle))
}

/// 按列优先读取 tile 段。`width` 为列数（`tiles_wide`），`height` 为行数（`tiles_high`）。
///
/// 返回长度 `width * height`，下标 `x * height + y`。
pub fn read_tiles(
    bytes: &[u8],
    version: u32,
    width: i32,
    height: i32,
    frame_important: &[bool],
) -> Result<Vec<WldCell>, WldError> {
    if version < 269 {
        return Err(WldError::UnsupportedVersion(version));
    }
    if width < 0 || height < 0 {
        return Err(WldError::BadSection);
    }
    let width = width as usize;
    let height = height as usize;
    let mut out = vec![WldCell::default(); width.saturating_mul(height)];
    let mut r = Rd::new(bytes);
    for x in 0..width {
        let mut y = 0usize;
        while y < height {
            let (cell, rle) = read_one_cell(&mut r, version, frame_important)?;
            out[x * height + y] = cell;
            let mut extra = rle;
            while extra > 0 {
                y += 1;
                if y >= height {
                    return Err(WldError::RleOverflow);
                }
                out[x * height + y] = cell;
                extra -= 1;
            }
            y += 1;
        }
    }
    let _ = r.rest();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_u16(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    fn push_i16(buf: &mut Vec<u8>, v: i16) {
        push_u16(buf, v as u16);
    }
    fn push_u32(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    fn push_i32(buf: &mut Vec<u8>, v: i32) {
        push_u32(buf, v as u32);
    }
    fn push_u64(buf: &mut Vec<u8>, v: u64) {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    fn push_i64(buf: &mut Vec<u8>, v: i64) {
        push_u64(buf, v as u64);
    }
    fn push_f64(buf: &mut Vec<u8>, v: f64) {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    fn push_str(buf: &mut Vec<u8>, s: &str) {
        assert!(s.len() < 128);
        buf.push(s.len() as u8);
        buf.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn reads_desktop_file_header() {
        let mut buf = Vec::new();
        push_u32(&mut buf, WORLD_VERSION_1_4_5_8);
        buf.extend_from_slice(b"relogic");
        buf.push(FILE_TYPE_WORLD);
        push_u32(&mut buf, 4);
        push_u64(&mut buf, 1);
        push_i16(&mut buf, 2);
        push_i32(&mut buf, 0);
        push_i32(&mut buf, 99);
        push_i16(&mut buf, 2);
        buf.push(0b0000_0010);
        let (header, n) = read_file_header(&buf).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(header.version, WORLD_VERSION_1_4_5_8);
        assert_eq!(&header.magic, b"relogic");
        assert!(header.favorite);
        assert_eq!(header.file_revision, 4);
        assert_eq!(header.tile_section_offset().unwrap(), 99);
        assert_eq!(header.frame_important, vec![false, true]);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut buf = Vec::new();
        push_u32(&mut buf, 326);
        buf.extend_from_slice(b"notawld");
        buf.push(2);
        assert_eq!(read_file_header(&buf).unwrap_err(), WldError::BadMagic);
    }

    #[test]
    fn reads_world_property_prefix() {
        let mut buf = Vec::new();
        push_str(&mut buf, "Small");
        push_str(&mut buf, "seed");
        push_u64(&mut buf, 9);
        buf.extend_from_slice(&[0u8; 16]);
        push_i32(&mut buf, 42);
        push_i32(&mut buf, 0);
        push_i32(&mut buf, 67200);
        push_i32(&mut buf, 0);
        push_i32(&mut buf, 19200);
        push_i32(&mut buf, 1200);
        push_i32(&mut buf, 4200);
        push_i32(&mut buf, 0);
        buf.extend_from_slice(&[0u8; 9]);
        push_i64(&mut buf, 1);
        push_i64(&mut buf, 2);
        buf.push(1);
        for _ in 0..(3 + 4 + 3 + 4 + 3) {
            push_i32(&mut buf, 0);
        }
        push_i32(&mut buf, 2100);
        push_i32(&mut buf, 320);
        push_f64(&mut buf, 380.5);
        push_f64(&mut buf, 900.0);
        push_f64(&mut buf, 0.0);
        buf.push(1);
        push_i32(&mut buf, 0);
        buf.push(0);
        buf.push(0);
        push_i32(&mut buf, 1);
        push_i32(&mut buf, 2);
        buf.push(1);
        let (props, n) = read_world_properties(&buf, WORLD_VERSION_1_4_5_8).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(props.title, "Small");
        assert_eq!(props.seed, "seed");
        assert_eq!(props.world_id, 42);
        assert_eq!(props.tiles_high, 1200);
        assert_eq!(props.tiles_wide, 4200);
        assert_eq!(props.spawn_x, 2100);
        assert_eq!(props.spawn_y, 320);
        assert!((props.ground_level - 380.5).abs() < 1e-9);
        assert!((props.rock_level - 900.0).abs() < 1e-9);
        assert!(props.crimson);
        assert!(!props.seeds.drunk);
    }

    #[test]
    fn reads_rle_column_and_framed_tile() {
        let mut plain = Vec::new();
        plain.push(0b0100_0010);
        plain.push(0);
        plain.push(1);
        let cells = read_tiles(&plain, 326, 1, 2, &[false]).unwrap();
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].tile_type, Some(0));
        assert_eq!(cells[1].tile_type, Some(0));
        assert_eq!(cells[0].frame_x, 0);

        let mut framed = Vec::new();
        framed.push(0b0000_0010);
        framed.push(5);
        push_i16(&mut framed, 18);
        push_i16(&mut framed, 36);
        let mut flags = vec![false; 6];
        flags[5] = true;
        let cells = read_tiles(&framed, 326, 1, 1, &flags).unwrap();
        assert_eq!(cells[0].tile_type, Some(5));
        assert_eq!((cells[0].frame_x, cells[0].frame_y), (18, 36));
    }

    #[test]
    fn reads_water_amount() {
        let mut buf = Vec::new();
        buf.push(0b0000_1010);
        buf.push(0);
        buf.push(200);
        let cells = read_tiles(&buf, 326, 1, 1, &[false]).unwrap();
        assert_eq!(cells[0].liquid_kind, LiquidKind::Water);
        assert_eq!(cells[0].liquid_amount, 200);
    }
}
