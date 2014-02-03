//! 世界与角色绘制（含遮挡光照）。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;
use tr_core::{BiomeId, BlockId, ItemId};

use crate::app::TerrariaApp;
use crate::lightmap::{LightMap, LightSample};
use crate::world::{
    TILE, WORLD_H, World, screen_len, screen_of, view_extent, wrap_delta_x, wrap_tx,
};

fn paint_cracks(draw: &mut DrawList, sx: f32, sy: f32, ratio: f32) {
    let a = (0.35 + ratio * 0.55).clamp(0.0, 0.9);
    let c = Color::rgba(0.05, 0.05, 0.08, a);
    let stage = (ratio * 3.0).ceil() as i32;
    draw.fill_rect(Rect::new(sx + 3.0, sy + 4.0, 8.0, 2.0), c);
    draw.fill_rect(Rect::new(sx + 9.0, sy + 8.0, 2.0, 7.0), c);
    if stage >= 2 {
        draw.fill_rect(Rect::new(sx + 2.0, sy + 12.0, 10.0, 2.0), c);
        draw.fill_rect(Rect::new(sx + 14.0, sy + 3.0, 2.0, 9.0), c);
    }
    if stage >= 3 {
        draw.fill_rect(Rect::new(sx + 5.0, sy + 15.0, 9.0, 2.0), c);
        draw.fill_rect(Rect::new(sx + 11.0, sy + 10.0, 6.0, 2.0), c);
    }
}

/// 材质色 × 彩色光照（通道可 >1 做局部过曝暖感）。
fn shade(c: Color, light: LightSample) -> Color {
    // 彩色光材质响应：主导通道略抬、非主导略压，避免只做均匀变暗。
    let mean = (light.r + light.g + light.b) * (1.0 / 3.0);
    let lift = 0.18;
    let r = c.r * light.r * (1.0 + (light.r - mean).max(0.0) * lift);
    let g = c.g * light.g * (1.0 + (light.g - mean).max(0.0) * lift);
    let b = c.b * light.b * (1.0 + (light.b - mean).max(0.0) * lift);
    Color::rgba(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), c.a)
}

fn tile_hash(tx: i32, ty: i32) -> u32 {
    let mut n = (tx as u32)
        .wrapping_mul(374761393)
        .wrapping_add((ty as u32).wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

/// 按格子镜像同一张贴图，打破整面墙的重复印章。
fn varied_uv(uv: Rect, tx: i32, ty: i32, flip_y: bool) -> Rect {
    let h = tile_hash(tx, ty);
    let mut uv = uv;
    if h & 1 != 0 {
        uv.x += uv.w;
        uv.w = -uv.w;
    }
    if flip_y && (h >> 1) & 1 != 0 {
        uv.y += uv.h;
        uv.h = -uv.h;
    }
    uv
}

fn stamp_can_flip_y(id: BlockId) -> bool {
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

#[derive(Clone, Copy)]
struct LightSrc {
    x: i32,
    y: i32,
    radius: i32,
    kind: BlockId,
}

fn collect_lights(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<LightSrc> {
    let mut out = Vec::new();
    let pad = 12;
    for ty in (y0 - pad).max(0)..=(y1 + pad).min(WORLD_H - 1) {
        for tx in (x0 - pad)..=(x1 + pad) {
            let id = world.get(tx, ty);
            if id.emits_light() {
                out.push(LightSrc {
                    x: wrap_tx(tx),
                    y: ty,
                    radius: id.light_radius(),
                    kind: id,
                });
            }
        }
    }
    out
}

/// 天空环境光（地表开敞处）。保留足够夜暗，让局部光源能塑形。
fn sky_ambient(dayness: f32) -> f32 {
    (0.16 + 0.84 * dayness).clamp(0.16, 1.0)
}

fn light_tint(kind: BlockId) -> Color {
    match kind {
        BlockId::TORCH => crate::palette::GLOW_TORCH,
        BlockId::FURNACE => crate::palette::GLOW_FURNACE,
        _ => crate::palette::GLOW_DEFAULT,
    }
}

/// 光源加性光晕（伪 bloom）：叠在瓦片之上、实体之前。
fn paint_light_halos(
    draw: &mut DrawList,
    cam_x: f32,
    cam_y: f32,
    lights: &[LightSrc],
    day_t: f32,
    atlas: Option<crate::tiles::TileView>,
) {
    for src in lights {
        let cx = src.x as f32 * TILE + TILE * 0.5 - cam_x;
        let cy = src.y as f32 * TILE + TILE * 0.35 - cam_y;
        let tint = light_tint(src.kind);
        let flicker = match src.kind {
            BlockId::TORCH | BlockId::FURNACE => {
                0.85 + 0.15 * (day_t * 9.0 + src.x as f32 * 0.7).sin()
            }
            _ => 1.0,
        };
        let base_r = src.radius as f32 * TILE * 0.55;
        if let Some(halo) = atlas {
            for (scale, a) in [(1.0, 0.10), (0.62, 0.14), (0.32, 0.22), (0.14, 0.32)] {
                let r = base_r * scale;
                draw.tex_rect(
                    halo.tex,
                    Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
                    halo.uv,
                    Color::rgba(tint.r, tint.g, tint.b, a * flicker),
                );
            }
        } else {
            for (scale, a) in [(1.0, 0.07), (0.62, 0.11), (0.32, 0.18), (0.14, 0.28)] {
                fill_disc(
                    draw,
                    cx,
                    cy,
                    base_r * scale,
                    Color::rgba(tint.r, tint.g, tint.b, a * flicker),
                );
            }
        }
    }
}

fn dayness_from_phase(phase: f32) -> f32 {
    if phase < 0.25 {
        phase / 0.25
    } else if phase < 0.55 {
        1.0
    } else if phase < 0.75 {
        1.0 - (phase - 0.55) / 0.2
    } else {
        0.0
    }
}

/// 近似实心圆（逐行扫描，层数少时开销可接受）。
fn fill_disc(draw: &mut DrawList, cx: f32, cy: f32, radius: f32, color: Color) {
    let r = radius.max(1.0);
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    for y in y0..=y1 {
        let dy = y as f32 + 0.5 - cy;
        let inner = r * r - dy * dy;
        if inner <= 0.0 {
            continue;
        }
        let half = inner.sqrt();
        draw.fill_rect(Rect::new(cx - half, y as f32, half * 2.0, 1.0), color);
    }
}

/// 交互物本体染色：火把 / 熔炉 / 箱子等。
fn paint_interactable_accents(
    draw: &mut DrawList,
    id: BlockId,
    sx: f32,
    sy: f32,
    day_t: f32,
    tx: i32,
    near: bool,
) {
    if near {
        // 靠近高亮：冷青外框，提示可交互。
        let pulse = 0.45 + 0.55 * (day_t * 4.0 + tx as f32).sin().abs();
        let a = 0.22 + 0.28 * pulse;
        let edge = crate::palette::HUD_ACCENT;
        draw.fill_rect(
            Rect::new(sx - 1.0, sy - 1.0, TILE + 2.0, 2.0),
            Color::rgba(edge.r, edge.g, edge.b, a),
        );
        draw.fill_rect(
            Rect::new(sx - 1.0, sy + TILE - 1.0, TILE + 2.0, 2.0),
            Color::rgba(edge.r, edge.g, edge.b, a * 0.85),
        );
        draw.fill_rect(
            Rect::new(sx - 1.0, sy, 2.0, TILE),
            Color::rgba(edge.r, edge.g, edge.b, a * 0.75),
        );
        draw.fill_rect(
            Rect::new(sx + TILE - 1.0, sy, 2.0, TILE),
            Color::rgba(edge.r, edge.g, edge.b, a * 0.75),
        );
    }
    match id {
        BlockId::TORCH => {
            let flicker = 0.75 + 0.25 * (day_t * 11.0 + tx as f32).sin();
            let glow = crate::palette::GLOW_TORCH;
            // 地面暖色泼溅，让附近土石读到火把色温。
            draw.fill_rect(
                Rect::new(sx + 2.0, sy + TILE - 3.0, TILE - 4.0, 2.0),
                Color::rgba(glow.r, glow.g * 0.7, glow.b * 0.35, 0.22 + 0.12 * flicker),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.28, sy + TILE * 0.08, TILE * 0.44, TILE * 0.18),
                Color::rgba(1.0, 0.85, 0.45, 0.2 + 0.15 * flicker),
            );
            // 上升烟丝：三点相位错开，弱化静态感。
            for i in 0..3 {
                let phase = day_t * (2.4 + i as f32 * 0.85) + tx as f32 * 0.37 + i as f32;
                let rise = (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let sway = (phase * 1.4).cos() * 3.5;
                let ox = sx + TILE * 0.42 + sway;
                let oy = sy - 2.0 - rise * 14.0;
                let a = (0.16 * (1.0 - rise) * flicker).clamp(0.0, 0.18);
                let s = 2.0 + rise * 2.5;
                draw.fill_rect(
                    Rect::new(ox, oy, s, s * 0.85),
                    Color::rgba(0.18, 0.14, 0.12, a),
                );
            }
        }
        BlockId::FURNACE => {
            let flicker = 0.8 + 0.2 * (day_t * 7.5 + tx as f32 * 0.4).sin();
            let glow = crate::palette::GLOW_FURNACE;
            // 炉口：深腔 + 橙心 + 外晕。
            draw.fill_rect(
                Rect::new(sx + TILE * 0.22, sy + TILE * 0.38, TILE * 0.56, TILE * 0.42),
                Color::rgba(0.08, 0.05, 0.04, 0.85),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.30, sy + TILE * 0.48, TILE * 0.40, TILE * 0.28),
                Color::rgba(glow.r, glow.g, glow.b, 0.55 + 0.3 * flicker),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.36, sy + TILE * 0.54, TILE * 0.28, TILE * 0.16),
                Color::rgba(1.0, 0.82, 0.35, 0.4 + 0.35 * flicker),
            );
            // 顶沿余烬
            draw.fill_rect(
                Rect::new(sx + 3.0, sy + 2.0, TILE - 6.0, 2.0),
                Color::rgba(glow.r, glow.g * 0.6, glow.b * 0.3, 0.25 + 0.15 * flicker),
            );
        }
        BlockId::CHEST => {
            // 箱盖缝与铜扣，让箱子和木板区分开。
            draw.fill_rect(
                Rect::new(sx + 2.0, sy + TILE * 0.42, TILE - 4.0, 2.0),
                Color::rgba(0.28, 0.16, 0.08, 0.85),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.42, sy + TILE * 0.48, TILE * 0.16, TILE * 0.16),
                Color::rgba(0.92, 0.78, 0.32, 0.9),
            );
        }
        BlockId::BED => {
            draw.fill_rect(
                Rect::new(sx + 2.0, sy + 3.0, TILE * 0.38, TILE * 0.28),
                Color::rgba(0.92, 0.9, 0.86, 0.85),
            );
            draw.fill_rect(
                Rect::new(sx + 2.0, sy + TILE - 4.0, TILE - 4.0, 2.0),
                Color::rgba(0.35, 0.18, 0.12, 0.7),
            );
        }
        BlockId::WORKBENCH => {
            let pulse = 0.5 + 0.5 * (day_t * 2.2 + tx as f32 * 0.2).sin();
            draw.fill_rect(
                Rect::new(sx + TILE * 0.18, sy + TILE * 0.22, TILE * 0.28, 3.0),
                Color::rgba(0.75, 0.78, 0.82, 0.7),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.55, sy + TILE * 0.18, 3.0, TILE * 0.35),
                Color::rgba(0.55, 0.32, 0.16, 0.75),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.62, sy + TILE * 0.55, TILE * 0.22, 2.0),
                Color::rgba(0.95, 0.72, 0.28, 0.25 + 0.2 * pulse),
            );
        }
        _ => {}
    }
}

/// 矿物自发光：暗处更显眼，亮处仍保留微弱脉动矿脉。
fn paint_ore_emissive(
    draw: &mut DrawList,
    id: BlockId,
    sx: f32,
    sy: f32,
    day_t: f32,
    tx: i32,
    ty: i32,
    light: LightSample,
) {
    let glow = match id {
        BlockId::COPPER_ORE => crate::palette::GLOW_COPPER,
        BlockId::IRON_ORE => crate::palette::GLOW_IRON,
        _ => return,
    };
    let pulse = 0.55 + 0.45 * (day_t * 2.6 + tx as f32 * 0.9 + ty as f32 * 0.4).sin();
    // 环境越暗，自发光占比越高。
    let dark = (1.0 - light.intensity()).clamp(0.0, 1.0);
    let a = 0.12 + 0.38 * dark * pulse + 0.08 * pulse;
    let spots = [
        (0.28, 0.30, 2.0),
        (0.62, 0.42, 2.5),
        (0.40, 0.68, 2.0),
        (0.72, 0.72, 1.5),
    ];
    for (fx, fy, r) in spots {
        let cx = sx + TILE * fx;
        let cy = sy + TILE * fy;
        draw.fill_rect(
            Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
            Color::rgba(glow.r, glow.g, glow.b, a),
        );
        // 外晕：弱扩散
        draw.fill_rect(
            Rect::new(cx - r * 1.8, cy - r * 1.8, r * 3.6, r * 3.6),
            Color::rgba(glow.r, glow.g, glow.b, a * 0.28),
        );
    }
    // 暗处矿脉局部雾：青蓝/暖橙薄纱，把晶体从洞穴底色里托出来。
    if dark > 0.35 {
        let mist = a * 0.22 * dark;
        draw.fill_rect(
            Rect::new(sx - 4.0, sy - 4.0, TILE + 8.0, TILE + 8.0),
            Color::rgba(glow.r, glow.g, glow.b, mist),
        );
    }
}

/// 中景群系剪影：随相机 X 卷动，穿越群系时轮廓变化。
fn paint_biome_backdrop(
    draw: &mut DrawList,
    world: &World,
    sw: f32,
    sh: f32,
    cam_x: f32,
    cam_y: f32,
    dayness: f32,
) {
    let py = -cam_y * 0.04;
    let base_y = sh * 0.68 + py;
    let a = 0.28 + 0.22 * dayness;
    let parallax = 0.22;
    let tx0 = ((cam_x * parallax) / TILE).floor() as i32 - 1;
    let tx1 = (((cam_x + sw) * parallax) / TILE).ceil() as i32 + 2;
    for wx in tx0..=tx1 {
        let biome = world.biome_at_x(wx);
        let sx = wx as f32 * TILE * parallax - cam_x * parallax;
        if sx < -40.0 || sx > sw + 40.0 {
            continue;
        }
        match biome {
            BiomeId::Forest => {
                let c = Color::rgba(0.05, 0.12, 0.07, a);
                let h = 40.0 + ((wx * 13).rem_euclid(7)) as f32 * 6.0;
                draw.fill_rect(Rect::new(sx, base_y - h, 10.0, h), c);
                draw.fill_rect(Rect::new(sx - 10.0, base_y - h - 18.0, 30.0, 22.0), c);
                draw.fill_rect(Rect::new(sx - 6.0, base_y - h - 32.0, 22.0, 16.0), c);
                // 树冠高光点，密林更「厚」。
                draw.fill_rect(
                    Rect::new(sx - 2.0, base_y - h - 28.0, 8.0, 4.0),
                    Color::rgba(0.18, 0.32, 0.14, a * 0.7),
                );
            }
            BiomeId::Desert => {
                let c = Color::rgba(0.28, 0.20, 0.10, a * 0.9);
                let h = 28.0 + ((wx * 9).rem_euclid(5)) as f32 * 5.0;
                draw.fill_rect(Rect::new(sx + 4.0, base_y - h, 6.0, h), c);
                draw.fill_rect(Rect::new(sx - 2.0, base_y - h * 0.55, 5.0, 12.0), c);
                draw.fill_rect(Rect::new(sx + 10.0, base_y - h * 0.7, 5.0, 10.0), c);
                // 沙丘弧线。
                draw.fill_rect(
                    Rect::new(sx - 8.0, base_y - 8.0, 28.0, 5.0),
                    Color::rgba(0.42, 0.32, 0.16, a * 0.55),
                );
            }
            BiomeId::Tundra => {
                let c = Color::rgba(0.14, 0.16, 0.22, a);
                let h = 22.0 + ((wx * 5).rem_euclid(4)) as f32 * 4.0;
                draw.fill_rect(Rect::new(sx, base_y - h, 36.0, h), c);
                draw.fill_rect(Rect::new(sx + 8.0, base_y - h - 10.0, 20.0, 12.0), c);
                draw.fill_rect(
                    Rect::new(sx + 10.0, base_y - h - 14.0, 16.0, 4.0),
                    Color::rgba(0.72, 0.82, 0.92, a * 0.55),
                );
            }
            BiomeId::Meadow => {
                let c = Color::rgba(0.08, 0.14, 0.08, a * 0.85);
                let h = 16.0 + ((wx * 3).rem_euclid(5)) as f32 * 3.0;
                draw.fill_rect(Rect::new(sx, base_y - h, 22.0, h), c);
                // 偶发花点。
                if (wx * 7).rem_euclid(5) == 0 {
                    draw.fill_rect(
                        Rect::new(sx + 6.0, base_y - h - 4.0, 3.0, 3.0),
                        Color::rgba(0.85, 0.55, 0.7, a * 0.6),
                    );
                }
            }
        }
    }
}

/// 地表薄雾：沿可见地表高度铺一层冷色半透明带，洞穴深处减弱。
fn paint_surface_fog(
    draw: &mut DrawList,
    world: &World,
    sw: f32,
    sh: f32,
    cam_x: f32,
    cam_y: f32,
    dayness: f32,
    player: &crate::player::Player,
) {
    let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
    let pty = (player.y / TILE).floor() as i32;
    let depth = (world.surface_at(ptx) - pty).max(0) as f32;
    // 深入地下时雾变薄，突出洞穴 vignette。
    let surface_weight = (1.0 - (depth / 14.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if surface_weight < 0.05 {
        return;
    }

    // 黄昏/黎明略加重，正午稍淡。
    let dusk = (1.0 - (dayness - 0.5).abs() * 2.0).clamp(0.0, 1.0);
    let base_a = (0.06 + 0.10 * dusk + 0.04 * (1.0 - dayness)) * surface_weight;

    let biome = world.biome_at_x(ptx);
    let (fog, fog_warm) = match biome {
        BiomeId::Meadow => (
            Color::rgba(0.55, 0.68, 0.82, base_a),
            Color::rgba(0.72, 0.58, 0.62, base_a * 0.55),
        ),
        BiomeId::Forest => (
            Color::rgba(0.42, 0.58, 0.48, base_a * 1.05),
            Color::rgba(0.35, 0.48, 0.38, base_a * 0.5),
        ),
        BiomeId::Desert => (
            Color::rgba(0.78, 0.62, 0.38, base_a * 0.95),
            Color::rgba(0.85, 0.48, 0.28, base_a * 0.45),
        ),
        BiomeId::Tundra => (
            Color::rgba(0.62, 0.74, 0.92, base_a * 1.1),
            Color::rgba(0.55, 0.62, 0.78, base_a * 0.4),
        ),
    };

    // 按列采样地表，画窄带雾，避免整屏糊死。
    let step = 8.0;
    let mut x = 0.0;
    while x < sw {
        let wx = wrap_tx(((cam_x + x + step * 0.5) / TILE).floor() as i32);
        let sy = world.surface_at(wx) as f32 * TILE - cam_y;
        // 雾带：地表上方一小段 + 贴地薄层。
        let band_top = sy - 28.0;
        let band_h = 36.0;
        if band_top < sh && band_top + band_h > 0.0 {
            let y0 = band_top.max(-4.0);
            let y1 = (band_top + band_h).min(sh + 4.0);
            let h = (y1 - y0).max(0.0);
            if h > 1.0 {
                draw.fill_rect(Rect::new(x, y0, step + 1.0, h * 0.55), fog);
                draw.fill_rect(Rect::new(x, y0 + h * 0.45, step + 1.0, h * 0.55), fog_warm);
            }
        }
        x += step;
    }

    // 近地全宽轻雾，把远景与地形接缝抹软。
    let horizon = world.surface_at(ptx) as f32 * TILE - cam_y;
    if horizon > -40.0 && horizon < sh + 40.0 {
        let soft = Color::rgba(fog.r, fog.g, fog.b, base_a * 0.45);
        draw.fill_rect(Rect::new(0.0, (horizon - 18.0).max(0.0), sw, 24.0), soft);
    }
}

/// 洞穴深处冷雾：玩家在地表以下时铺黑蓝半透明层，与地表雾互斥加重。
fn paint_cave_fog(
    draw: &mut DrawList,
    world: &World,
    sw: f32,
    sh: f32,
    player: &crate::player::Player,
) {
    let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
    let pty = (player.y / TILE).floor() as i32;
    let depth = (world.surface_at(ptx) - pty).max(0) as f32;
    let cave = ((depth - 3.0) / 16.0).clamp(0.0, 1.0);
    if cave < 0.08 {
        return;
    }
    let a = 0.05 + 0.14 * cave;
    // 全屏冷雾 + 底部更重的一层，塑造纵深。
    draw.fill_rect(
        Rect::new(0.0, 0.0, sw, sh),
        Color::rgba(0.04, 0.08, 0.18, a * 0.55),
    );
    draw.fill_rect(
        Rect::new(0.0, sh * 0.55, sw, sh * 0.45),
        Color::rgba(0.02, 0.04, 0.12, a),
    );
}

/// 群系浮尘：花粉 / 沙粒 / 雪晶，仅地表附近稀疏点缀。
fn paint_biome_motes(
    draw: &mut DrawList,
    world: &World,
    sw: f32,
    sh: f32,
    cam_x: f32,
    cam_y: f32,
    day_t: f32,
    player: &crate::player::Player,
) {
    let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
    let pty = (player.y / TILE).floor() as i32;
    let depth = (world.surface_at(ptx) - pty).max(0) as f32;
    if depth > 8.0 {
        return;
    }
    let biome = world.biome_at_x(ptx);
    let n = match biome {
        BiomeId::Meadow => 14,
        BiomeId::Forest => 18,
        BiomeId::Desert => 16,
        BiomeId::Tundra => 12,
    };
    for i in 0..n {
        let seed = (i as f32 * 17.13 + ptx as f32 * 0.31).sin() * 43758.5453;
        let fx = seed.fract();
        let fy = ((i as f32 * 9.7 + day_t * 0.15).sin() * 0.5 + 0.5).fract();
        let sx = fx * sw;
        let drift = match biome {
            BiomeId::Desert => (day_t * 12.0 + i as f32).sin() * 18.0,
            BiomeId::Tundra => (day_t * 4.0 + i as f32).cos() * 6.0,
            _ => (day_t * 6.0 + i as f32 * 1.7).sin() * 10.0,
        };
        let wx = wrap_tx(((cam_x + sx + drift) / TILE).floor() as i32);
        let surf = world.surface_at(wx) as f32 * TILE - cam_y;
        let sy = surf - 8.0 - fy * 70.0;
        if sy < -4.0 || sy > sh {
            continue;
        }
        let (c, size) = match biome {
            BiomeId::Meadow => (Color::rgba(0.85, 0.92, 0.55, 0.28), 2.0),
            BiomeId::Forest => (Color::rgba(0.55, 0.85, 0.40, 0.22), 2.2),
            BiomeId::Desert => (Color::rgba(0.92, 0.78, 0.48, 0.32), 1.8),
            BiomeId::Tundra => (Color::rgba(0.88, 0.94, 1.0, 0.35), 2.4),
        };
        let bob = (day_t * 3.0 + i as f32).sin() * 2.0;
        draw.fill_rect(Rect::new(sx + drift * 0.15, sy + bob, size, size), c);
    }
}

/// 群系天气：密林雨丝、寒地落雪、荒原热浪。只在近地表画。
fn paint_biome_weather(
    draw: &mut DrawList,
    world: &World,
    sw: f32,
    sh: f32,
    _cam_x: f32,
    cam_y: f32,
    day_t: f32,
    dayness: f32,
    player: &crate::player::Player,
) {
    let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
    let pty = (player.y / TILE).floor() as i32;
    let depth = (world.surface_at(ptx) - pty).max(0) as f32;
    if depth > 6.0 {
        return;
    }
    let biome = world.biome_at_x(ptx);
    match biome {
        BiomeId::Forest if dayness < 0.62 => {
            let fall = (day_t * 90.0) % 18.0;
            for i in 0..28 {
                let x = ((i as f32 * 47.0 + day_t * 12.0) % sw).abs();
                let y = ((i as f32 * 31.0 + fall * (6.0 + (i % 3) as f32)) % sh).abs();
                let a = 0.10 + 0.08 * (1.0 - dayness);
                draw.fill_rect(Rect::new(x, y, 1.0, 8.0), Color::rgba(0.72, 0.82, 0.95, a));
            }
        }
        BiomeId::Tundra => {
            let fall = (day_t * 28.0) % 16.0;
            for i in 0..18 {
                let x = ((i as f32 * 71.0 + day_t * 6.0) % sw).abs();
                let y = ((i as f32 * 43.0 + fall * 4.0) % (sh * 0.72)).abs();
                let s = if i % 3 == 0 { 3.0 } else { 2.0 };
                draw.fill_rect(Rect::new(x, y, s, s), Color::rgba(0.92, 0.96, 1.0, 0.28));
            }
        }
        BiomeId::Desert if dayness > 0.5 => {
            let horizon = world.surface_at(ptx) as f32 * TILE - cam_y;
            let y0 = (horizon - 36.0).max(0.0);
            let heat = ((dayness - 0.5) / 0.5).clamp(0.0, 1.0);
            for i in 0..5 {
                let wave = (day_t * 3.0 + i as f32).sin() * 6.0;
                let y = y0 + i as f32 * 7.0 + wave;
                if y > sh {
                    continue;
                }
                draw.fill_rect(
                    Rect::new(0.0, y, sw, 1.0),
                    Color::rgba(0.95, 0.78, 0.42, 0.06 * heat),
                );
            }
        }
        _ => {}
    }
}

/// 挥击短暂弧光：朝面向一侧扇形短条。
fn paint_swing_arc(draw: &mut DrawList, player: &crate::player::Player, cam_x: f32, cam_y: f32) {
    let flash = player.swing_flash();
    if flash < 0.35 {
        return;
    }
    let fade = ((flash - 0.35) / 0.65).clamp(0.0, 1.0);
    let dir = if player.facing >= 0.0 { 1.0 } else { -1.0 };
    let px = cam_x + wrap_delta_x(cam_x, player.x);
    let cx = px + crate::player::HIT_W * 0.5 - cam_x + dir * 10.0;
    let cy = player.y + crate::player::HIT_H * 0.45 - cam_y;
    let a = 0.35 * fade;
    let item = player.selected_item();
    let tint = if item.melee_damage().is_some() || item.is_tool() {
        Color::rgba(1.0, 0.92, 0.55, a)
    } else {
        Color::rgba(0.85, 0.9, 1.0, a * 0.7)
    };
    for i in 0..5 {
        let t = i as f32 / 4.0;
        let ang = (t - 0.5) * 1.1;
        let reach = 18.0 + fade * 10.0;
        let ox = cx + dir * ang.cos() * reach;
        let oy = cy + ang.sin() * reach * 0.85;
        let s = 3.0 + (1.0 - (t - 0.5).abs() * 2.0) * 2.0;
        draw.fill_rect(Rect::new(ox - s * 0.5, oy - s * 0.5, s, s * 0.7), tint);
    }
}

impl TerrariaApp {
    pub(crate) fn draw_world(&self, draw: &mut DrawList) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let Some(player) = self.player.as_ref() else {
            return;
        };

        let phase = self.day_t / 180.0;
        let dayness = dayness_from_phase(phase);
        let sky_amb = sky_ambient(dayness);
        let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
        let biome = world.biome_at_x(ptx);
        crate::sky::paint_sky_parallax(
            draw,
            &self.sky_atlas,
            sw,
            sh,
            self.cam_x,
            self.cam_y,
            dayness,
            biome,
        );
        paint_biome_backdrop(draw, world, sw, sh, self.cam_x, self.cam_y, dayness);

        let view_w = view_extent(sw);
        let view_h = view_extent(sh);
        let tile_px = screen_len(TILE);
        let x0 = (self.cam_x / TILE).floor() as i32 - 1;
        let y0 = (self.cam_y / TILE).floor() as i32 - 1;
        let x1 = ((self.cam_x + view_w) / TILE).ceil() as i32 + 1;
        let y1 = ((self.cam_y + view_h) / TILE).ceil() as i32 + 1;
        let lights = collect_lights(world, x0, y0, x1, y1);
        let lmap = LightMap::build(world, x0, y0, x1, y1, sky_amb, dayness);

        // 背景墙整层先画。后面的实心块盖住同一格；平台、火把、家具不再挡住墙。
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let wall = world.get_wall(tx, ty);
                let Some((r, g, b)) = wall.color() else {
                    continue;
                };
                let light = lmap.sample(tx, ty);
                let sx = screen_of(tx as f32 * TILE, self.cam_x);
                let sy = screen_of(ty as f32 * TILE, self.cam_y);
                if let Some(view) = self.tile_atlas.wall_framed(world, tx, ty) {
                    draw.tex_rect(
                        view.tex,
                        Rect::new(sx, sy, tile_px, tile_px),
                        view.uv,
                        shade(Color::rgb(1.0, 1.0, 1.0), light),
                    );
                } else if let Some(view) = self.tile_atlas.wall(wall) {
                    draw.tex_rect(
                        view.tex,
                        Rect::new(sx, sy, tile_px, tile_px),
                        varied_uv(view.uv, tx, ty, true),
                        shade(Color::rgb(1.0, 1.0, 1.0), light),
                    );
                } else {
                    draw.fill_rect(
                        Rect::new(sx, sy, tile_px, tile_px),
                        shade(Color::rgb(r, g, b), light),
                    );
                }
                let crack = world.wall_damage_ratio(tx, ty);
                if crack > 0.05 {
                    if let Some(view) = self.tile_atlas.crack() {
                        draw.tex_rect(
                            view.tex,
                            Rect::new(sx, sy, tile_px, tile_px),
                            view.uv,
                            Color::rgba(1.0, 1.0, 1.0, (0.25 + crack * 0.45).clamp(0.0, 0.75)),
                        );
                    } else {
                        paint_cracks(draw, sx, sy, crack * 0.85);
                    }
                }
            }
        }

        // 自然树在实心物块之前（背景层）：人走在树前，放置的方块盖在树冠前。
        self.tree_atlas.paint_behind_solids(
            draw,
            world,
            self.cam_x,
            self.cam_y,
            x0,
            y0,
            x1,
            y1,
            |tx, ty| shade(Color::rgb(1.0, 1.0, 1.0), lmap.sample(tx, ty)),
        );

        // 液体后层：墙和树之后、实心物块之前。
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if world.get(tx, ty) != BlockId::WATER {
                    continue;
                }
                let light = lmap.sample(tx, ty);
                let sx = screen_of(tx as f32 * TILE, self.cam_x);
                let sy = screen_of(ty as f32 * TILE, self.cam_y);
                self.draw_water_body(draw, world, tx, ty, sx, sy, light);
            }
        }

        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let id = world.get(tx, ty);
                if id == BlockId::AIR {
                    // 空格加性余晖：洞穴里火把旁更醒目。
                    let light = lmap.sample(tx, ty);
                    let amb = sky_amb * 0.35;
                    if light.intensity() > amb + 0.06 {
                        let glow = ((light.intensity() - amb) * 0.45).clamp(0.0, 0.32);
                        let sx = screen_of(tx as f32 * TILE, self.cam_x);
                        let sy = screen_of(ty as f32 * TILE, self.cam_y);
                        // 空格余晖跟局部光照色温，不再死橙。
                        draw.fill_rect(
                            Rect::new(sx, sy, tile_px, tile_px),
                            Color::rgba(light.r, light.g * 0.85, light.b * 0.55, glow),
                        );
                    }
                    continue;
                }
                // 自然树已由背景层绘制；液体已画；遗留假叶不画。
                if id.is_tree()
                    || id == BlockId::LEAF
                    || id == BlockId::WATER
                    || (self.tree_atlas.ready() && crate::trees::hides_block(world, tx, ty))
                {
                    continue;
                }
                let light = lmap.sample(tx, ty);
                let sx = screen_of(tx as f32 * TILE, self.cam_x);
                let sy = screen_of(ty as f32 * TILE, self.cam_y);

                let drawn_atlas = if let Some(view) = self.tile_atlas.block_framed(world, tx, ty) {
                    // 地形 framing 已含变体；家具等仍可轻微镜像打破印章。
                    let uv = if matches!(
                        id,
                        BlockId::DIRT
                            | BlockId::GRASS
                            | BlockId::STONE
                            | BlockId::SAND
                            | BlockId::SNOW
                            | BlockId::COPPER_ORE
                            | BlockId::IRON_ORE
                    ) {
                        view.uv
                    } else {
                        varied_uv(view.uv, tx, ty, stamp_can_flip_y(id))
                    };
                    draw.tex_rect(
                        view.tex,
                        Rect::new(sx, sy, TILE, TILE),
                        uv,
                        shade(Color::rgb(1.0, 1.0, 1.0), light),
                    );
                    true
                } else {
                    false
                };

                if !drawn_atlas {
                    use crate::palette::{DIRT, GRASS, STONE};
                    let color = match id {
                        BlockId::DIRT => DIRT.base,
                        BlockId::GRASS => GRASS.base,
                        BlockId::STONE => STONE.base,
                        BlockId::WOOD => Color::rgb(0.55, 0.35, 0.18),
                        id if id.is_tree() => Color::rgb(0.55, 0.35, 0.18),
                        BlockId::LEAF => Color::rgba(0.22, 0.55, 0.22, 0.85),
                        BlockId::WORKBENCH => Color::rgb(0.62, 0.42, 0.22),
                        BlockId::SAPLING => Color::rgb(0.35, 0.72, 0.28),
                        BlockId::TORCH => Color::rgb(0.55, 0.32, 0.12),
                        BlockId::PLATFORM => Color::rgb(0.5, 0.32, 0.16),
                        BlockId::CHEST => Color::rgb(0.58, 0.38, 0.18),
                        BlockId::LADDER => Color::rgb(0.48, 0.3, 0.14),
                        BlockId::ROPE => Color::rgb(0.68, 0.5, 0.26),
                        BlockId::SAND => Color::rgb(0.82, 0.72, 0.42),
                        BlockId::SNOW => Color::rgb(0.88, 0.92, 0.96),
                        BlockId::COPPER_ORE => Color::rgb(0.72, 0.48, 0.28),
                        BlockId::IRON_ORE => Color::rgb(0.55, 0.58, 0.62),
                        BlockId::FURNACE => Color::rgb(0.35, 0.32, 0.30),
                        BlockId::BED => Color::rgb(0.7, 0.25, 0.3),
                        _ => Color::rgb(1.0, 0.0, 1.0),
                    };
                    if id == BlockId::LADDER {
                        draw.fill_rect(
                            Rect::new(sx + TILE * 0.28, sy, TILE * 0.12, TILE),
                            shade(Color::rgb(0.55, 0.35, 0.16), light),
                        );
                        draw.fill_rect(
                            Rect::new(sx + TILE * 0.6, sy, TILE * 0.12, TILE),
                            shade(Color::rgb(0.55, 0.35, 0.16), light),
                        );
                        for k in 0..3 {
                            let ry = sy + 4.0 + k as f32 * (TILE / 3.0);
                            draw.fill_rect(
                                Rect::new(sx + TILE * 0.28, ry, TILE * 0.44, 3.0),
                                shade(Color::rgb(0.7, 0.48, 0.22), light),
                            );
                        }
                    } else if id == BlockId::ROPE {
                        draw.fill_rect(
                            Rect::new(sx + TILE * 0.42, sy, TILE * 0.16, TILE),
                            shade(Color::rgb(0.72, 0.55, 0.28), light),
                        );
                        for k in 0..4 {
                            let ry = sy + 2.0 + k as f32 * (TILE / 4.0);
                            draw.fill_rect(
                                Rect::new(sx + TILE * 0.34, ry, TILE * 0.32, 2.0),
                                shade(Color::rgb(0.55, 0.40, 0.18), light),
                            );
                        }
                    } else if id == BlockId::PLATFORM {
                        draw.fill_rect(
                            Rect::new(sx, sy + TILE * 0.55, TILE, TILE * 0.35),
                            shade(Color::rgb(0.62, 0.4, 0.2), light),
                        );
                        draw.fill_rect(
                            Rect::new(sx, sy + TILE * 0.55, TILE, 3.0),
                            shade(Color::rgb(0.8, 0.55, 0.3), light),
                        );
                    } else {
                        draw.fill_rect(Rect::new(sx, sy, tile_px, tile_px), shade(color, light));
                    }
                }

                if id == BlockId::TORCH && !drawn_atlas {
                    let flicker = 0.75 + 0.25 * (self.day_t * 11.0 + tx as f32).sin();
                    draw.fill_rect(
                        Rect::new(sx + TILE * 0.42, sy + TILE * 0.35, TILE * 0.16, TILE * 0.5),
                        shade(Color::rgb(0.4, 0.25, 0.12), light),
                    );
                    draw.fill_rect(
                        Rect::new(sx + TILE * 0.32, sy + TILE * 0.12, TILE * 0.36, TILE * 0.32),
                        Color::rgba(1.0, 0.7, 0.25, 0.55 + 0.35 * flicker),
                    );
                }
                if matches!(
                    id,
                    BlockId::TORCH
                        | BlockId::FURNACE
                        | BlockId::CHEST
                        | BlockId::BED
                        | BlockId::WORKBENCH
                ) {
                    let ptx =
                        wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
                    let pty = ((player.y + crate::player::HIT_H * 0.5) / TILE).floor() as i32;
                    let dx = wrap_delta_x(ptx as f32 * TILE, tx as f32 * TILE).abs() / TILE;
                    let dy = (pty - ty).abs() as f32;
                    let near = dx <= 2.5 && dy <= 2.0;
                    paint_interactable_accents(draw, id, sx, sy, self.day_t, tx, near);
                }
                if matches!(id, BlockId::COPPER_ORE | BlockId::IRON_ORE) {
                    paint_ore_emissive(draw, id, sx, sy, self.day_t, tx, ty, light);
                }
                if id == BlockId::SAPLING {
                    if let Some(left) = world.sapling_left(tx, ty) {
                        let ratio = (1.0 - left / 20.0).clamp(0.0, 1.0);
                        draw.fill_rect(
                            Rect::new(sx + 2.0, sy + TILE - 3.0, (TILE - 4.0) * ratio, 2.0),
                            Color::rgb(0.4, 0.9, 0.5),
                        );
                    }
                }

                crate::terrain_edges::paint_tile_edges(draw, world, tx, ty, id, sx, sy, light);

                let crack = world.damage_ratio(tx, ty);
                if crack > 0.05 {
                    if let Some(view) = self.tile_atlas.crack() {
                        draw.tex_rect(
                            view.tex,
                            Rect::new(sx, sy, tile_px, tile_px),
                            view.uv,
                            Color::rgba(1.0, 1.0, 1.0, (0.35 + crack * 0.55).clamp(0.0, 0.9)),
                        );
                    } else {
                        paint_cracks(draw, sx, sy, crack);
                    }
                }
            }
        }

        // 液体前层：水面高光与落水条纹，盖在物块边缘上，仍在实体之前。
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if world.get(tx, ty) != BlockId::WATER {
                    continue;
                }
                let light = lmap.sample(tx, ty);
                let sx = screen_of(tx as f32 * TILE, self.cam_x);
                let sy = screen_of(ty as f32 * TILE, self.cam_y);
                self.draw_water_front(draw, world, tx, ty, sx, sy, light);
            }
        }

        paint_light_halos(
            draw,
            self.cam_x,
            self.cam_y,
            &lights,
            self.day_t,
            self.tile_atlas.halo(),
        );

        paint_surface_fog(draw, world, sw, sh, self.cam_x, self.cam_y, dayness, player);
        paint_cave_fog(draw, world, sw, sh, player);

        for d in &world.drops {
            let sx = screen_of(d.x, self.cam_x);
            let bob = screen_len((d.bob * 6.0).sin() * 2.0);
            let sy = screen_of(d.y, self.cam_y) + bob;
            let tx = (d.x / TILE).floor() as i32;
            let ty = (d.y / TILE).floor() as i32;
            let amb = lmap.sample(tx, ty);
            let light = amb;
            let c = match d.item {
                ItemId::DIRT => Color::rgb(0.55, 0.38, 0.22),
                ItemId::STONE => Color::rgb(0.55, 0.58, 0.62),
                ItemId::WOOD => Color::rgb(0.7, 0.48, 0.25),
                ItemId::WORKBENCH => Color::rgb(0.8, 0.55, 0.3),
                ItemId::SAPLING => Color::rgb(0.4, 0.85, 0.35),
                ItemId::WOOD_PICK | ItemId::STONE_PICK => Color::rgb(0.7, 0.7, 0.75),
                ItemId::WOOD_SWORD => Color::rgb(0.85, 0.75, 0.4),
                ItemId::TORCH => Color::rgb(1.0, 0.7, 0.25),
                ItemId::GEL => Color::rgb(0.75, 0.35, 0.85),
                ItemId::PLATFORM => Color::rgb(0.7, 0.48, 0.25),
                ItemId::CHEST => Color::rgb(0.85, 0.6, 0.28),
                ItemId::LADDER => Color::rgb(0.65, 0.42, 0.2),
                ItemId::WOOD_ARMOR => Color::rgb(0.55, 0.4, 0.25),
                _ => Color::rgb(1.0, 1.0, 0.4),
            };
            draw.fill_rect(Rect::new(sx, sy, 8.0, 8.0), shade(c, light));
            draw.fill_rect(
                Rect::new(sx + 1.0, sy + 1.0, 6.0, 2.0),
                Color::rgba(1.0, 1.0, 1.0, 0.35),
            );
        }

        player.draw(draw, self.cam_x, self.cam_y, &self.player_atlas);
        paint_swing_arc(draw, player, self.cam_x, self.cam_y);
        crate::grapple::draw(player, draw, self.cam_x, self.cam_y);
        for npc in &self.town_npcs {
            npc.draw(draw, self.cam_x, self.cam_y, &self.npc_atlas);
        }
        if self.house_flash_t > 0.0 {
            let a = (self.house_flash_t / 2.5).clamp(0.0, 1.0) * 0.35;
            let c = if self.house_query_ok {
                Color::rgba(0.35, 0.85, 0.45, a)
            } else {
                Color::rgba(0.95, 0.45, 0.35, a)
            };
            for &(tx, ty) in &self.house_query_tiles {
                let sx = screen_of(tx as f32 * TILE, self.cam_x);
                let sy = screen_of(ty as f32 * TILE, self.cam_y);
                draw.fill_rect(Rect::new(sx, sy, tile_px, tile_px), c);
            }
        }
        for e in &self.enemies {
            e.draw(
                draw,
                self.cam_x,
                self.cam_y,
                self.tile_atlas.slime(),
                self.npc_atlas.zombie(),
                self.npc_atlas.cell_size(),
                self.npc_atlas.demon_eye(),
                self.npc_atlas.demon_eye_cell(),
            );
        }
        crate::weapon::draw_projectiles(&self.projectiles, draw, self.cam_x, self.cam_y);
        crate::fx::draw_floaters(&self.damage_fx, draw, self.cam_x, self.cam_y);
        crate::fx::draw_dust(&self.dust_fx, draw, self.cam_x, self.cam_y);
        paint_biome_motes(
            draw, world, sw, sh, self.cam_x, self.cam_y, self.day_t, player,
        );
        paint_biome_weather(
            draw, world, sw, sh, self.cam_x, self.cam_y, self.day_t, dayness, player,
        );

        let (mx, my) = self.mouse;
        let menus_block_aim = self.craft_open || self.bag_open || self.chest_open.is_some();
        if !menus_block_aim {
            if let Some(preview) =
                crate::aim::evaluate(world, player, mx + self.cam_x, my + self.cam_y)
            {
                crate::aim::paint(draw, self.cam_x, self.cam_y, preview);
            }
        }

        // 后期叠层：盖住世界与角色，不盖 HUD。
        {
            let ptx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
            let pty = (player.y / TILE).floor() as i32;
            let depth = (world.surface_at(ptx) - pty).max(0) as f32;
            let cave = ((depth - 4.0) / 18.0).clamp(0.0, 1.0);
            crate::postprocess::paint_cave_vignette(draw, sw, sh, cave);
        }
        crate::postprocess::paint_color_grade(draw, sw, sh, dayness);
        // iframes 刚触发时接近 0.7，映射成闪屏强度。
        let hurt_t = ((player.iframes - 0.35) / 0.35).clamp(0.0, 1.0);
        crate::postprocess::paint_hurt_flash(draw, sw, sh, hurt_t);
    }

    /// 液体后层：格内水体，不含水面高光。
    fn draw_water_body(
        &self,
        draw: &mut DrawList,
        world: &World,
        tx: i32,
        ty: i32,
        sx: f32,
        sy: f32,
        light: LightSample,
    ) {
        let lv = world.fluid_level(tx, ty);
        let fill = lv.fill_ratio().clamp(0.05, 1.0);
        let tile_px = screen_len(TILE);
        let h = tile_px * fill;
        let top = sy + tile_px - h;
        let deep = lv.is_source() || lv.is_falling();
        let base_a = if deep { 0.58 } else { 0.45 };
        let body = shade(
            Color::rgba(
                crate::palette::WATER.base.r,
                crate::palette::WATER.base.g,
                crate::palette::WATER.base.b,
                base_a,
            ),
            light.max_with(0.35),
        );
        draw.fill_rect(Rect::new(sx, top, tile_px, h), body);
        if world.get(tx, ty - 1) == BlockId::WATER {
            draw.fill_rect(
                Rect::new(sx, sy, tile_px, tile_px),
                shade(crate::palette::WATER.shade, light.max_with(0.3)),
            );
        }
    }

    /// 液体前层：水面高光、倒影和落水条纹。
    fn draw_water_front(
        &self,
        draw: &mut DrawList,
        world: &World,
        tx: i32,
        ty: i32,
        sx: f32,
        sy: f32,
        light: LightSample,
    ) {
        let lv = world.fluid_level(tx, ty);
        let fill = lv.fill_ratio().clamp(0.05, 1.0);
        let tile_px = screen_len(TILE);
        let h = tile_px * fill;
        let top = sy + tile_px - h;
        let above = world.get(tx, ty - 1);
        if above != BlockId::WATER {
            let wave = (self.day_t * 2.4 + tx as f32 * 0.7 + ty as f32 * 0.3).sin();
            let hl = crate::palette::WATER.highlight;
            let highlight = Color::rgba(
                hl.r + 0.08 * wave,
                hl.g + 0.05 * wave,
                hl.b,
                0.38 + 0.12 * wave,
            );
            draw.fill_rect(Rect::new(sx, top, TILE, 2.0f32.max(h.min(3.0))), highlight);
            // 浅流时顶沿略斜：邻格水位差 → 右缘微调
            if !lv.is_source() && !lv.is_falling() {
                let right = world.fluid_level(tx + 1, ty);
                let left = world.fluid_level(tx - 1, ty);
                let slope = (right.fill_ratio() - left.fill_ratio()) * 2.0;
                if slope.abs() > 0.05 {
                    let tip_h = 2.0;
                    let tip_y = top + slope.clamp(-2.0, 2.0);
                    draw.fill_rect(
                        Rect::new(sx + TILE * 0.55, tip_y, TILE * 0.45, tip_h),
                        Color::rgba(0.7, 0.88, 1.0, 0.25),
                    );
                }
            }
            // 简易天光反射：顶面亮条 + 向水体内衰减的竖向色带。
            let sky_tint = Color::rgba(
                (0.45 + light.r * 0.45).min(1.0),
                (0.55 + light.g * 0.35).min(1.0),
                (0.75 + light.b * 0.25).min(1.0),
                1.0,
            );
            let reflect_a = 0.10 + 0.12 * light.intensity();
            draw.fill_rect(
                Rect::new(sx + 1.0, top + 1.0, TILE - 2.0, 1.0),
                Color::rgba(sky_tint.r, sky_tint.g, sky_tint.b, reflect_a + 0.08),
            );
            // 竖向反射条：模拟天空倒影沉入水体。
            let reflect_depth = (h - 3.0).clamp(2.0, TILE * 0.65);
            const BANDS: i32 = 4;
            for i in 0..BANDS {
                let t = (i as f32 + 0.5) / BANDS as f32;
                let y = top + 2.0 + reflect_depth * t;
                let a = reflect_a * (1.0 - t) * (0.55 + 0.2 * wave);
                if a < 0.02 {
                    continue;
                }
                let inset = 2.0 + t * 3.0;
                draw.fill_rect(
                    Rect::new(sx + inset, y, TILE - inset * 2.0, 1.0),
                    Color::rgba(sky_tint.r, sky_tint.g, sky_tint.b, a),
                );
            }
            // 横向波纹高光：随时间与格坐标错开。
            let shimmer_x = sx + 3.0 + ((wave + 1.0) * 0.5) * (TILE - 8.0);
            draw.fill_rect(
                Rect::new(shimmer_x, top + 2.0, 3.0, 1.0),
                Color::rgba(0.92, 0.97, 1.0, 0.18 + 0.1 * wave.abs()),
            );
        }

        if lv.is_falling() {
            // 下落水：竖直条纹暗示流动
            let pulse = 0.5 + 0.5 * (self.day_t * 6.0 + tx as f32).sin();
            draw.fill_rect(
                Rect::new(sx + TILE * 0.35, sy, TILE * 0.12, TILE),
                Color::rgba(0.65, 0.85, 1.0, 0.12 + 0.1 * pulse),
            );
            draw.fill_rect(
                Rect::new(sx + TILE * 0.58, sy, TILE * 0.1, TILE),
                Color::rgba(0.55, 0.78, 0.98, 0.1 + 0.08 * pulse),
            );
        }
    }
}
