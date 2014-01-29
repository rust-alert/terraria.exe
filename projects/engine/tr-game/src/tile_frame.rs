//! 正版方块图集邻接分帧（base / dirt-blend）。
//!
//! 规则表与 TerraFirma 公开 UV 规则同构：18 像素步长、16 像素采样格、
//! 16 位双比特邻接掩码。结论自用；不把参考仓路径写进本模块。

use spark_core::Rect;
use tr_core::{BlockId, WallId};

use crate::world::World;

/// 图集步长：16 像素画面 + 2 像素间距。
pub const STRIDE: u32 = 18;
/// 实际绘制采样边长。
pub const CELL: u32 = 16;

/// 一条 UV 规则：`(mask & rule.mask) == rule.val` 时取三组变体之一。
#[derive(Clone, Copy)]
struct UvRule {
    mask: u16,
    val: u16,
    /// 像素坐标：`(u0,v0,u1,v1,u2,v2)`。
    uvs: [u16; 6],
}

/// 同族 / 泥土融合用的 base 区规则。
const BASE_RULES: &[UvRule] = &[
    UvRule {
        mask: 0x50ff,
        val: 0x00ff,
        uvs: [108, 18, 126, 18, 144, 18],
    },
    UvRule {
        mask: 0x05ff,
        val: 0x00ff,
        uvs: [108, 36, 126, 36, 144, 36],
    },
    UvRule {
        mask: 0x44ff,
        val: 0x00ff,
        uvs: [180, 0, 180, 18, 180, 36],
    },
    UvRule {
        mask: 0x11ff,
        val: 0x00ff,
        uvs: [198, 0, 198, 18, 198, 36],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ff,
        uvs: [18, 18, 36, 18, 54, 18],
    },
    UvRule {
        mask: 0x007f,
        val: 0x003f,
        uvs: [18, 0, 36, 0, 54, 0],
    },
    UvRule {
        mask: 0x00df,
        val: 0x00cf,
        uvs: [18, 36, 36, 36, 54, 36],
    },
    UvRule {
        mask: 0x00f7,
        val: 0x00f3,
        uvs: [0, 0, 0, 18, 0, 36],
    },
    UvRule {
        mask: 0x00fd,
        val: 0x00fc,
        uvs: [72, 0, 72, 18, 72, 36],
    },
    UvRule {
        mask: 0x0077,
        val: 0x0033,
        uvs: [0, 54, 36, 54, 72, 54],
    },
    UvRule {
        mask: 0x007d,
        val: 0x003c,
        uvs: [18, 54, 54, 54, 90, 54],
    },
    UvRule {
        mask: 0x00d7,
        val: 0x00c3,
        uvs: [0, 72, 36, 72, 72, 72],
    },
    UvRule {
        mask: 0x00dd,
        val: 0x00cc,
        uvs: [18, 72, 54, 72, 90, 72],
    },
    UvRule {
        mask: 0x00f5,
        val: 0x00f0,
        uvs: [90, 0, 90, 18, 90, 36],
    },
    UvRule {
        mask: 0x005f,
        val: 0x000f,
        uvs: [108, 72, 126, 72, 144, 72],
    },
    UvRule {
        mask: 0x0075,
        val: 0x0030,
        uvs: [108, 0, 126, 0, 144, 0],
    },
    UvRule {
        mask: 0x00d5,
        val: 0x00c0,
        uvs: [108, 54, 126, 54, 144, 54],
    },
    UvRule {
        mask: 0x0057,
        val: 0x0003,
        uvs: [162, 0, 162, 18, 162, 36],
    },
    UvRule {
        mask: 0x005d,
        val: 0x000c,
        uvs: [216, 0, 216, 18, 216, 36],
    },
    UvRule {
        mask: 0x0055,
        val: 0x0000,
        uvs: [162, 54, 180, 54, 198, 54],
    },
    UvRule {
        mask: 0x0000,
        val: 0x0000,
        uvs: [18, 18, 36, 18, 54, 18],
    },
];

/// 泥土与石头族交界的 blend 区。
const BLEND_RULES: &[UvRule] = &[
    UvRule {
        mask: 0x00ff,
        val: 0x00bf,
        uvs: [144, 108, 162, 108, 180, 108],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ef,
        uvs: [144, 90, 162, 90, 180, 90],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00fb,
        uvs: [162, 126, 162, 144, 162, 162],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00fe,
        uvs: [144, 126, 144, 144, 144, 162],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00bb,
        uvs: [36, 90, 36, 126, 36, 162],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00be,
        uvs: [54, 90, 54, 126, 54, 162],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00eb,
        uvs: [36, 108, 36, 144, 36, 180],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ee,
        uvs: [54, 108, 54, 144, 54, 180],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00fa,
        uvs: [180, 126, 180, 144, 180, 162],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00af,
        uvs: [144, 180, 162, 180, 180, 180],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ba,
        uvs: [198, 90, 198, 108, 198, 126],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ea,
        uvs: [216, 144, 216, 162, 216, 180],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00ab,
        uvs: [216, 90, 216, 108, 216, 126],
    },
    UvRule {
        mask: 0x00ff,
        val: 0x00aa,
        uvs: [108, 198, 126, 198, 144, 198],
    },
    UvRule {
        mask: 0x03ff,
        val: 0x02ff,
        uvs: [0, 90, 0, 126, 0, 162],
    },
    UvRule {
        mask: 0x0cff,
        val: 0x08ff,
        uvs: [18, 90, 18, 126, 18, 162],
    },
    UvRule {
        mask: 0x30ff,
        val: 0x20ff,
        uvs: [0, 108, 0, 144, 0, 180],
    },
    UvRule {
        mask: 0xc0ff,
        val: 0x80ff,
        uvs: [18, 108, 18, 144, 18, 180],
    },
];

/// 非草泥土融合补充规则。
const NO_GRASS_RULES: &[UvRule] = &[
    UvRule {
        mask: 0x00fb,
        val: 0x00b3,
        uvs: [72, 144, 72, 162, 72, 180],
    },
    UvRule {
        mask: 0x00fb,
        val: 0x00e3,
        uvs: [72, 90, 72, 108, 72, 126],
    },
    UvRule {
        mask: 0x00fe,
        val: 0x00bc,
        uvs: [90, 144, 90, 162, 90, 180],
    },
    UvRule {
        mask: 0x00fe,
        val: 0x00ec,
        uvs: [90, 90, 90, 108, 90, 126],
    },
    UvRule {
        mask: 0x00bf,
        val: 0x003b,
        uvs: [0, 198, 18, 198, 36, 198],
    },
    UvRule {
        mask: 0x00bf,
        val: 0x003e,
        uvs: [54, 198, 72, 198, 90, 198],
    },
    UvRule {
        mask: 0x00ef,
        val: 0x00cb,
        uvs: [0, 216, 18, 216, 36, 216],
    },
    UvRule {
        mask: 0x00ef,
        val: 0x00ce,
        uvs: [54, 216, 72, 216, 90, 216],
    },
    UvRule {
        mask: 0x00fa,
        val: 0x00a0,
        uvs: [108, 216, 108, 234, 108, 252],
    },
    UvRule {
        mask: 0x00ca,
        val: 0x0080,
        uvs: [126, 144, 126, 162, 126, 180],
    },
    UvRule {
        mask: 0x003a,
        val: 0x0020,
        uvs: [126, 90, 126, 108, 126, 126],
    },
    UvRule {
        mask: 0x00af,
        val: 0x000a,
        uvs: [162, 198, 180, 198, 198, 198],
    },
    UvRule {
        mask: 0x00ac,
        val: 0x0008,
        uvs: [0, 252, 18, 252, 36, 252],
    },
    UvRule {
        mask: 0x00a3,
        val: 0x0002,
        uvs: [54, 252, 72, 252, 90, 252],
    },
    UvRule {
        mask: 0x00ea,
        val: 0x0080,
        uvs: [108, 144, 108, 162, 108, 180],
    },
    UvRule {
        mask: 0x00ba,
        val: 0x0020,
        uvs: [108, 90, 108, 108, 108, 126],
    },
    UvRule {
        mask: 0x00ae,
        val: 0x0008,
        uvs: [0, 234, 18, 234, 36, 234],
    },
    UvRule {
        mask: 0x00ab,
        val: 0x0002,
        uvs: [54, 234, 72, 234, 90, 234],
    },
    UvRule {
        mask: 0x00bf,
        val: 0x002f,
        uvs: [234, 0, 252, 0, 270, 0],
    },
    UvRule {
        mask: 0x00ef,
        val: 0x008f,
        uvs: [234, 18, 252, 18, 270, 18],
    },
    UvRule {
        mask: 0x00fb,
        val: 0x00f2,
        uvs: [234, 36, 252, 36, 270, 36],
    },
    UvRule {
        mask: 0x00fe,
        val: 0x00f8,
        uvs: [234, 54, 252, 54, 270, 54],
    },
];

/// 邻接语义：同族 / 泥土融合 / 空气。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rel {
    Same,
    Blend,
    Open,
}

fn is_stone_family(id: BlockId) -> bool {
    matches!(id, BlockId::STONE | BlockId::COPPER_ORE | BlockId::IRON_ORE)
}

fn is_dirtish(id: BlockId) -> bool {
    matches!(id, BlockId::DIRT | BlockId::GRASS)
}

fn is_terrain_framed(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::DIRT
            | BlockId::GRASS
            | BlockId::STONE
            | BlockId::SAND
            | BlockId::SNOW
            | BlockId::COPPER_ORE
            | BlockId::IRON_ORE
    )
}

fn uses_dirt_blend(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::DIRT
            | BlockId::STONE
            | BlockId::SAND
            | BlockId::SNOW
            | BlockId::COPPER_ORE
            | BlockId::IRON_ORE
    )
}

fn classify(center: BlockId, other: BlockId) -> Rel {
    if other == BlockId::AIR || other == BlockId::WATER {
        return Rel::Open;
    }
    if other == center {
        return Rel::Same;
    }
    // 草与泥土视为同族，方便地表衔接。
    if (center == BlockId::GRASS && other == BlockId::DIRT)
        || (center == BlockId::DIRT && other == BlockId::GRASS)
    {
        return Rel::Same;
    }
    if is_stone_family(center) && is_stone_family(other) {
        return Rel::Same;
    }
    // 泥土 / 沙 / 雪 对石头族：融合边。
    if uses_dirt_blend(center) && is_dirtish(other) && center != BlockId::GRASS {
        // 非草泥土块把邻居泥土标成 blend（与 TerraFirma dirt 旗同向）。
        if is_dirtish(center) {
            return Rel::Blend;
        }
    }
    if is_dirtish(center) && is_stone_family(other) {
        return Rel::Blend;
    }
    if is_stone_family(center) && is_dirtish(other) {
        return Rel::Blend;
    }
    if matches!(center, BlockId::SAND | BlockId::SNOW) && is_dirtish(other) {
        return Rel::Blend;
    }
    if is_dirtish(center) && matches!(other, BlockId::SAND | BlockId::SNOW) {
        return Rel::Blend;
    }
    Rel::Open
}

fn pack_dir(rel: Rel, same_bits: u16, blend_bits: u16) -> u16 {
    match rel {
        Rel::Same => same_bits,
        Rel::Blend => blend_bits,
        Rel::Open => 0,
    }
}

fn build_mask(world: &World, tx: i32, ty: i32, center: BlockId) -> u16 {
    let t = classify(center, world.get(tx, ty - 1));
    let b = classify(center, world.get(tx, ty + 1));
    let l = classify(center, world.get(tx - 1, ty));
    let r = classify(center, world.get(tx + 1, ty));
    let tl = classify(center, world.get(tx - 1, ty - 1));
    let tr = classify(center, world.get(tx + 1, ty - 1));
    let bl = classify(center, world.get(tx - 1, ty + 1));
    let br = classify(center, world.get(tx + 1, ty + 1));

    let mut mask = 0u16;
    mask |= pack_dir(t, 0xc0, 0x80);
    mask |= pack_dir(b, 0x30, 0x20);
    mask |= pack_dir(l, 0x0c, 0x08);
    mask |= pack_dir(r, 0x03, 0x02);
    mask |= pack_dir(tl, 0xc000, 0x8000);
    mask |= pack_dir(tr, 0x3000, 0x2000);
    mask |= pack_dir(bl, 0x0c00, 0x0800);
    mask |= pack_dir(br, 0x0300, 0x0200);
    mask
}

fn variant_index(tx: i32, ty: i32) -> usize {
    // 与原版确定性变体一致：((x*7)+(y*11)) % 3
    let v = ((tx.wrapping_mul(7)).wrapping_add(ty.wrapping_mul(11))).rem_euclid(3) as usize;
    v * 2
}

fn match_rules(rules: &[UvRule], mask: u16, set: usize) -> Option<(u16, u16)> {
    for rule in rules {
        if mask & rule.mask == rule.val {
            return Some((rule.uvs[set], rule.uvs[set + 1]));
        }
    }
    None
}

/// 计算图集像素左上角 `(u, v)`。不可 framing 的方块返回 `None`。
pub fn frame_uv_px(world: &World, tx: i32, ty: i32) -> Option<(u16, u16)> {
    let id = world.get(tx, ty);
    if !is_terrain_framed(id) {
        return None;
    }
    let mut mask = build_mask(world, tx, ty, id);
    let set = variant_index(tx, ty);

    if uses_dirt_blend(id) {
        if let Some(uv) = match_rules(BLEND_RULES, mask, set) {
            return Some(uv);
        }
        if id != BlockId::GRASS {
            if let Some(uv) = match_rules(NO_GRASS_RULES, mask, set) {
                return Some(uv);
            }
        }
    }

    // 草：把 blend 位升成 same，再走 base。
    if id == BlockId::GRASS {
        mask |= (mask & 0xaaaa) >> 1;
    }

    match_rules(BASE_RULES, mask, set)
}

/// 把像素帧换成归一化 UV。`sheet_w/h` 为整张图集尺寸。
pub fn frame_to_uv(u: u16, v: u16, sheet_w: u32, sheet_h: u32) -> Option<Rect> {
    if sheet_w < CELL || sheet_h < CELL {
        return None;
    }
    let ux = u as u32;
    let uy = v as u32;
    if ux + CELL > sheet_w || uy + CELL > sheet_h {
        return None;
    }
    Some(Rect::new(
        ux as f32 / sheet_w as f32,
        uy as f32 / sheet_h as f32,
        CELL as f32 / sheet_w as f32,
        CELL as f32 / sheet_h as f32,
    ))
}

fn wall_neighbor(world: &World, tx: i32, ty: i32, center: WallId) -> Rel {
    let other = world.get_wall(tx, ty);
    if other == WallId::NONE {
        return Rel::Open;
    }
    if other == center {
        Rel::Same
    } else {
        // 异种墙仍接边，走 same 以免满是空洞。
        Rel::Same
    }
}

fn build_wall_mask(world: &World, tx: i32, ty: i32, center: WallId) -> u16 {
    let t = wall_neighbor(world, tx, ty - 1, center);
    let b = wall_neighbor(world, tx, ty + 1, center);
    let l = wall_neighbor(world, tx - 1, ty, center);
    let r = wall_neighbor(world, tx + 1, ty, center);
    let tl = wall_neighbor(world, tx - 1, ty - 1, center);
    let tr = wall_neighbor(world, tx + 1, ty - 1, center);
    let bl = wall_neighbor(world, tx - 1, ty + 1, center);
    let br = wall_neighbor(world, tx + 1, ty + 1, center);

    let mut mask = 0u16;
    mask |= pack_dir(t, 0xc0, 0x80);
    mask |= pack_dir(b, 0x30, 0x20);
    mask |= pack_dir(l, 0x0c, 0x08);
    mask |= pack_dir(r, 0x03, 0x02);
    mask |= pack_dir(tl, 0xc000, 0x8000);
    mask |= pack_dir(tr, 0x3000, 0x2000);
    mask |= pack_dir(bl, 0x0c00, 0x0800);
    mask |= pack_dir(br, 0x0300, 0x0200);
    mask
}

/// 背景墙图集 UV（复用 base 规则表）。
pub fn wall_frame_uv_px(world: &World, tx: i32, ty: i32) -> Option<(u16, u16)> {
    let id = world.get_wall(tx, ty);
    if id == WallId::NONE {
        return None;
    }
    let mask = build_wall_mask(world, tx, ty, id);
    let set = variant_index(tx, ty);
    match_rules(BASE_RULES, mask, set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_tile_uses_open_uv() {
        // mask=0 表示四向皆空，应命中 `0x0055/0x0000`，而非落到末尾兜底。
        let (u, v) = match_rules(BASE_RULES, 0, 0).unwrap();
        assert_eq!((u, v), (162, 54));
    }

    #[test]
    fn catch_all_base_rule_is_last() {
        let last = BASE_RULES.last().expect("base rules");
        assert_eq!(last.mask, 0);
        assert_eq!(last.val, 0);
        assert_eq!((last.uvs[0], last.uvs[1]), (18, 18));
    }

    #[test]
    fn variant_is_stable() {
        assert_eq!(variant_index(0, 0), 0);
        assert_eq!(variant_index(1, 0), 2);
        assert_eq!(variant_index(2, 0), 4);
    }
}
