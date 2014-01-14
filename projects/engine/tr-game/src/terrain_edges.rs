//! 邻居感知地形边缘叠层（顶 / 底 / 左右 / 草悬边 / 水岸）。
//!
//! 不改瓦片 UV，只在已绘制方块上叠加细边，把阶梯剪影做软。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::BlockId;

use crate::lightmap::LightSample;
use crate::palette;
use crate::world::{TILE, World};

fn shade(c: Color, light: LightSample) -> Color {
    Color::rgba(
        (c.r * light.r).clamp(0.0, 1.0),
        (c.g * light.g).clamp(0.0, 1.0),
        (c.b * light.b).clamp(0.0, 1.0),
        c.a,
    )
}

fn is_open(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::AIR
            | BlockId::WATER
            | BlockId::LEAF
            | BlockId::SAPLING
            | BlockId::TORCH
            | BlockId::LADDER
            | BlockId::ROPE
            | BlockId::PLATFORM
    )
}

fn is_terrain(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::DIRT
            | BlockId::GRASS
            | BlockId::STONE
            | BlockId::SCRAP
            | BlockId::SAND
            | BlockId::SNOW
            | BlockId::COPPER_ORE
            | BlockId::IRON_ORE
            | BlockId::WOOD
    )
}

fn is_stoneish(id: BlockId) -> bool {
    matches!(
        id,
        BlockId::STONE | BlockId::COPPER_ORE | BlockId::IRON_ORE | BlockId::SCRAP
    )
}

fn hash2(x: i32, y: i32) -> u32 {
    let mut n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

/// 在单个地形格上绘制邻居边缘。
pub fn paint_tile_edges(
    draw: &mut DrawList,
    world: &World,
    tx: i32,
    ty: i32,
    id: BlockId,
    sx: f32,
    sy: f32,
    light: LightSample,
) {
    if !is_terrain(id) {
        return;
    }

    let above = world.get(tx, ty - 1);
    let below = world.get(tx, ty + 1);
    let left = world.get(tx - 1, ty);
    let right = world.get(tx + 1, ty);

    let open_above = is_open(above);
    let open_below = is_open(below);
    let open_left = is_open(left);
    let open_right = is_open(right);
    let water_left = left == BlockId::WATER;
    let water_right = right == BlockId::WATER;
    let water_below = below == BlockId::WATER;
    let water_above = above == BlockId::WATER;

    // 悬空底边：3px 接触阴影 + 角部落石/泥土。
    if open_below {
        draw.fill_rect(
            Rect::new(sx, sy + TILE - 3.0, TILE, 3.0),
            shade(Color::rgba(0.02, 0.02, 0.04, 0.58), light.max_with(0.2)),
        );
        let hbits = hash2(tx, ty);
        let drip_l = 2.0 + (hbits % 3) as f32;
        let drip_r = 2.0 + ((hbits >> 3) % 3) as f32;
        let crumb = if is_stoneish(id) {
            Color::rgba(0.12, 0.12, 0.14, 0.55)
        } else {
            Color::rgba(
                palette::DIRT.rim.r,
                palette::DIRT.rim.g,
                palette::DIRT.rim.b,
                0.7,
            )
        };
        if open_left || (hbits & 1) == 0 {
            draw.fill_rect(
                Rect::new(sx, sy + TILE, 2.0, drip_l),
                shade(crumb, light.max_with(0.15)),
            );
        }
        if open_right || (hbits & 2) == 0 {
            draw.fill_rect(
                Rect::new(sx + TILE - 2.0, sy + TILE, 2.0, drip_r),
                shade(crumb, light.max_with(0.15)),
            );
        }
        // 中部落一点碎屑，打破直线底边。
        if (hbits >> 5) % 3 == 0 {
            let ox = sx + 4.0 + ((hbits >> 8) % 8) as f32;
            draw.fill_rect(
                Rect::new(ox, sy + TILE, 1.5, 1.5 + ((hbits >> 11) % 2) as f32),
                shade(crumb, light.max_with(0.12)),
            );
        }

        // 洞穴顶 AO 唇边：石/土悬空时更暗的内沿。
        if matches!(id, BlockId::DIRT | BlockId::STONE | BlockId::GRASS) || is_stoneish(id) {
            draw.fill_rect(
                Rect::new(sx + 1.0, sy + TILE - 5.0, TILE - 2.0, 2.0),
                shade(Color::rgba(0.01, 0.01, 0.03, 0.4), light.max_with(0.12)),
            );
        }
    }

    // 暴露顶面。
    if open_above {
        if id == BlockId::GRASS {
            draw.fill_rect(
                Rect::new(sx, sy, TILE, 2.0),
                shade(
                    Color::rgba(
                        palette::GRASS.highlight.r,
                        palette::GRASS.highlight.g,
                        palette::GRASS.highlight.b,
                        0.45,
                    ),
                    light,
                ),
            );
            // 草叶断续竖条，高度随格变化。
            for i in 0..4 {
                let hbits = hash2(tx + i, ty);
                let bh = 2.0 + (hbits % 3) as f32;
                let bx = sx + 2.0 + i as f32 * 4.0 + ((tx + i) % 2) as f32;
                draw.fill_rect(
                    Rect::new(bx, sy - bh + 1.0, 1.5, bh),
                    shade(
                        Color::rgba(
                            palette::GRASS.base.r,
                            palette::GRASS.base.g,
                            palette::GRASS.base.b,
                            0.55,
                        ),
                        light,
                    ),
                );
            }
        } else {
            draw.fill_rect(
                Rect::new(sx, sy, TILE, 2.0),
                shade(Color::rgba(0.02, 0.02, 0.05, 0.35), light.max_with(0.15)),
            );
        }
    }

    // 左右立面：压暗外露侧边。
    let side_a = if id == BlockId::GRASS { 0.28 } else { 0.40 };
    if open_left {
        draw.fill_rect(
            Rect::new(sx, sy, 2.0, TILE),
            shade(Color::rgba(0.02, 0.02, 0.04, side_a), light.max_with(0.12)),
        );
        if id == BlockId::GRASS && open_above {
            // 左侧不规则悬草。
            let hbits = hash2(tx * 3, ty);
            let len = 2.0 + (hbits % 4) as f32;
            let thick = 1.5 + ((hbits >> 2) % 2) as f32;
            draw.fill_rect(
                Rect::new(
                    sx - len,
                    sy + 1.0 + ((hbits >> 4) % 2) as f32,
                    len + 1.0,
                    thick,
                ),
                shade(
                    Color::rgba(
                        palette::GRASS.base.r,
                        palette::GRASS.base.g,
                        palette::GRASS.base.b,
                        0.55,
                    ),
                    light,
                ),
            );
            if (hbits & 4) != 0 {
                draw.fill_rect(
                    Rect::new(sx - len * 0.6, sy + 3.0, len * 0.55, 1.5),
                    shade(
                        Color::rgba(
                            palette::GRASS.shade.r,
                            palette::GRASS.shade.g,
                            palette::GRASS.shade.b,
                            0.45,
                        ),
                        light,
                    ),
                );
            }
        }
    }
    if open_right {
        draw.fill_rect(
            Rect::new(sx + TILE - 2.0, sy, 2.0, TILE),
            shade(Color::rgba(0.02, 0.02, 0.04, side_a), light.max_with(0.12)),
        );
        if id == BlockId::GRASS && open_above {
            let hbits = hash2(tx * 5 + 1, ty);
            let len = 2.0 + (hbits % 4) as f32;
            let thick = 1.5 + ((hbits >> 2) % 2) as f32;
            draw.fill_rect(
                Rect::new(
                    sx + TILE - 1.0,
                    sy + 1.0 + ((hbits >> 4) % 2) as f32,
                    len + 1.0,
                    thick,
                ),
                shade(
                    Color::rgba(
                        palette::GRASS.base.r,
                        palette::GRASS.base.g,
                        palette::GRASS.base.b,
                        0.55,
                    ),
                    light,
                ),
            );
            if (hbits & 4) != 0 {
                draw.fill_rect(
                    Rect::new(sx + TILE, sy + 3.0, len * 0.55, 1.5),
                    shade(
                        Color::rgba(
                            palette::GRASS.shade.r,
                            palette::GRASS.shade.g,
                            palette::GRASS.shade.b,
                            0.45,
                        ),
                        light,
                    ),
                );
            }
        }
    }

    // 水岸泡沫：邻格是水时点几颗亮像素。
    if water_left || water_right || water_below || water_above {
        let foam = Color::rgba(
            palette::WATER.highlight.r,
            palette::WATER.highlight.g,
            palette::WATER.highlight.b,
            0.7,
        );
        let hbits = hash2(tx + 17, ty + 9);
        if water_left {
            for i in 0..3 {
                let oy = sy + 4.0 + i as f32 * 4.0 + ((hbits >> i) % 3) as f32;
                draw.fill_rect(
                    Rect::new(sx - 1.0, oy, 2.0, 1.5),
                    shade(foam, light.max_with(0.35)),
                );
            }
        }
        if water_right {
            for i in 0..3 {
                let oy = sy + 3.0 + i as f32 * 4.0 + ((hbits >> (i + 3)) % 3) as f32;
                draw.fill_rect(
                    Rect::new(sx + TILE - 1.0, oy, 2.0, 1.5),
                    shade(foam, light.max_with(0.35)),
                );
            }
        }
        if water_below {
            for i in 0..3 {
                let ox = sx + 3.0 + i as f32 * 4.0 + ((hbits >> (i + 6)) % 2) as f32;
                draw.fill_rect(
                    Rect::new(ox, sy + TILE - 2.0, 2.0, 2.0),
                    shade(foam, light.max_with(0.35)),
                );
            }
        }
        if water_above {
            for i in 0..2 {
                let ox = sx + 4.0 + i as f32 * 6.0;
                draw.fill_rect(
                    Rect::new(ox, sy, 2.0, 2.0),
                    shade(foam, light.max_with(0.3)),
                );
            }
        }
    }

    // 石/土与空气交界的内凹角。
    if open_above && open_left {
        draw.fill_rect(
            Rect::new(sx, sy, 3.0, 3.0),
            shade(Color::rgba(0.02, 0.02, 0.05, 0.25), light.max_with(0.1)),
        );
    }
    if open_above && open_right {
        draw.fill_rect(
            Rect::new(sx + TILE - 3.0, sy, 3.0, 3.0),
            shade(Color::rgba(0.02, 0.02, 0.05, 0.25), light.max_with(0.1)),
        );
    }
}
