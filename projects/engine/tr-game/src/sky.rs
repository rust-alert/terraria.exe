//! 视差天空。贴图路径来自内容表；缺图时用程序绘制兜底。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};
use tr_core::BiomeId;

use crate::palette;

/// 单层天空贴图。
#[derive(Debug, Clone, Copy)]
pub struct SkyLayer {
    pub tex: TextureId,
    pub w: u32,
    pub h: u32,
}

/// 已上传的天空贴图层；缺层时对应绘制走程序兜底。
#[derive(Debug, Default)]
pub struct SkyAtlas {
    pub backdrop: Option<SkyLayer>,
    pub stars: Option<SkyLayer>,
    pub body: Option<SkyLayer>,
    pub clouds: Option<SkyLayer>,
    pub hills: [Option<SkyLayer>; 4],
    ready: bool,
}

impl SkyAtlas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        if let Some(layer) = upload_xnb_layer(draw, assets.forest_background.as_deref()) {
            self.backdrop = Some(layer);
        }
        self.backdrop = upload_layer(draw, assets.sky_backdrop.as_deref());
        self.stars = upload_layer(draw, assets.sky_stars.as_deref());
        self.body = upload_layer(draw, assets.sky_body.as_deref());
        self.clouds = upload_layer(draw, assets.sky_clouds.as_deref());
        self.hills = [
            upload_layer(draw, assets.sky_hills_meadow.as_deref()),
            upload_layer(draw, assets.sky_hills_forest.as_deref()),
            upload_layer(draw, assets.sky_hills_desert.as_deref()),
            upload_layer(draw, assets.sky_hills_tundra.as_deref()),
        ];
        let n = [
            self.backdrop.is_some(),
            self.stars.is_some(),
            self.body.is_some(),
            self.clouds.is_some(),
            self.hills.iter().any(|h| h.is_some()),
        ]
        .into_iter()
        .filter(|x| *x)
        .count();
        if n == 0 {
            tracing::info!("天空贴图未找到，使用程序兜底");
        } else {
            tracing::info!(layers = n, "天空贴图层已上传");
        }
    }

    fn hills_for(&self, biome: BiomeId) -> Option<SkyLayer> {
        let i = match biome {
            BiomeId::Meadow => 0,
            BiomeId::Forest => 1,
            BiomeId::Desert => 2,
            BiomeId::Tundra => 3,
        };
        self.hills[i]
    }
}

fn upload_xnb_layer(draw: &mut DrawList, path: Option<&std::path::Path>) -> Option<SkyLayer> {
    let path = path?;
    let tex = crate::xnb::decode_texture_file(path).ok()?;
    match draw.create_texture(tex.width, tex.height, tex.rgba) {
        Ok(gpu) => Some(SkyLayer {
            tex: gpu,
            w: tex.width,
            h: tex.height,
        }),
        Err(e) => {
            tracing::warn!(?e, path = %path.display(), "天空 XNB 上传失败");
            None
        }
    }
}

fn upload_layer(draw: &mut DrawList, path: Option<&std::path::Path>) -> Option<SkyLayer> {
    let path = path?;
    let (w, h, rgba) = crate::content_boot::ContentAssets::load_rgba(path)?;
    match draw.create_texture(w, h, rgba) {
        Ok(tex) => Some(SkyLayer { tex, w, h }),
        Err(e) => {
            tracing::warn!(?e, path = %path.display(), "天空贴图上传失败");
            None
        }
    }
}

/// 绘制完整天空背景（世界瓦片之下）。有 PNG 用贴图，缺层再程序补。
pub fn paint_sky_parallax(
    draw: &mut DrawList,
    atlas: &SkyAtlas,
    sw: f32,
    sh: f32,
    cam_x: f32,
    cam_y: f32,
    dayness: f32,
    biome: BiomeId,
) {
    let band = match biome {
        BiomeId::Meadow => palette::SKY_MEADOW,
        BiomeId::Forest => palette::SKY_FOREST,
        BiomeId::Desert => palette::SKY_DESERT,
        BiomeId::Tundra => palette::SKY_TUNDRA,
    };

    let px_far = -cam_x * 0.025;
    let py_far = -cam_y * 0.01;
    let px_mid = -cam_x * 0.08;
    let py_mid = -cam_y * 0.02;
    let px_near = -cam_x * 0.16;
    let py_near = -cam_y * 0.035;

    if let Some(layer) = atlas.backdrop {
        let tint = Color::rgb(
            0.35 + 0.65 * dayness,
            0.38 + 0.62 * dayness,
            0.55 + 0.45 * dayness,
        );
        blit_cover(draw, layer, sw, sh, tint);
    } else {
        paint_sky_gradient(draw, sw, sh, dayness, band);
    }

    let night = (1.0 - dayness).clamp(0.0, 1.0);
    if let Some(layer) = atlas.stars {
        if night > 0.05 {
            let a = 0.2 + 0.75 * night;
            blit_scroll(
                draw,
                layer,
                sw,
                sh * 0.55,
                0.0,
                px_far * 0.4,
                py_far,
                Color::rgba(1.0, 1.0, 1.0, a),
            );
        }
    } else {
        paint_stars(draw, sw, sh, px_far, py_far, dayness);
    }

    if let Some(layer) = atlas.body {
        let a = 0.35 + 0.55 * night;
        let dw = (sw * 0.42).min(layer.w as f32 * 2.0);
        let dh = dw * (layer.h as f32 / (layer.w as f32).max(1.0));
        let x = sw * 0.58 + px_far;
        let y = sh * 0.08 + py_far;
        draw.tex_rect(
            layer.tex,
            Rect::new(x, y, dw, dh),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::rgba(1.0, 1.0, 1.0, a),
        );
    } else {
        paint_planets(draw, sw, sh, px_far, py_far, dayness);
    }

    if let Some(layer) = atlas.hills_for(biome) {
        let y = sh * 0.52 + py_mid;
        let h = sh * 0.38;
        blit_scroll(
            draw,
            layer,
            sw,
            h,
            y,
            px_mid,
            0.0,
            Color::rgba(1.0, 1.0, 1.0, 0.55 + 0.25 * dayness),
        );
    } else {
        paint_distant_ridges(draw, sw, sh, px_mid, py_mid, dayness, band.ridge);
    }

    if let Some(layer) = atlas.clouds {
        let a = 0.25 + 0.45 * dayness;
        blit_scroll(
            draw,
            layer,
            sw,
            sh * 0.28,
            sh * 0.12 + py_near,
            px_near,
            0.0,
            Color::rgba(1.0, 1.0, 1.0, a),
        );
    } else {
        paint_cloud_layers(draw, sw, sh, px_near, py_near, dayness, biome);
    }

    // 最近山脊：有群系 hills 贴图时再近一层；否则程序剪影。
    if let Some(layer) = atlas.hills_for(biome) {
        let y = sh * 0.68 + py_near;
        let h = sh * 0.34;
        blit_scroll(
            draw,
            layer,
            sw,
            h,
            y,
            px_near * 1.35,
            0.0,
            Color::rgba(0.85, 0.88, 0.92, 0.35 + 0.2 * dayness),
        );
    } else {
        paint_near_hills(draw, sw, sh, px_near, py_near, band.hill);
    }
}

fn blit_cover(draw: &mut DrawList, layer: SkyLayer, sw: f32, sh: f32, tint: Color) {
    draw.tex_rect(
        layer.tex,
        Rect::new(0.0, 0.0, sw, sh),
        Rect::new(0.0, 0.0, 1.0, 1.0),
        tint,
    );
}

/// 横向循环铺贴（视差卷动）。
fn blit_scroll(
    draw: &mut DrawList,
    layer: SkyLayer,
    sw: f32,
    band_h: f32,
    y: f32,
    scroll_x: f32,
    _scroll_y: f32,
    tint: Color,
) {
    let aspect = layer.w as f32 / (layer.h as f32).max(1.0);
    let tile_w = (band_h * aspect).max(sw * 0.35);
    let mut x = -scroll_x.rem_euclid(tile_w);
    if x > 0.0 {
        x -= tile_w;
    }
    while x < sw + tile_w {
        draw.tex_rect(
            layer.tex,
            Rect::new(x, y, tile_w, band_h),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            tint,
        );
        x += tile_w;
    }
}

fn paint_sky_gradient(draw: &mut DrawList, sw: f32, sh: f32, dayness: f32, band: palette::SkyBand) {
    let night = palette::SKY_NIGHT_TOP;
    let top = mix_rgb(night, band.day_top, dayness);
    let mid = mix_rgb(band.dusk, palette::SKY_DAY_MID, dayness);
    let bot = mix_rgb(palette::SKY_NIGHT_BOT, band.day_bot, dayness);

    const BANDS: i32 = 18;
    for i in 0..BANDS {
        let t0 = i as f32 / BANDS as f32;
        let t1 = (i + 1) as f32 / BANDS as f32;
        let c = if t0 < 0.42 {
            mix_rgb(top, mid, t0 / 0.42)
        } else {
            mix_rgb(mid, bot, (t0 - 0.42) / 0.58)
        };
        draw.fill_rect(Rect::new(0.0, sh * t0, sw, sh * (t1 - t0) + 1.0), c);
    }
}

fn paint_stars(draw: &mut DrawList, sw: f32, sh: f32, px: f32, py: f32, dayness: f32) {
    let night = (1.0 - dayness).clamp(0.0, 1.0);
    if night < 0.08 {
        return;
    }
    let a = 0.15 + 0.55 * night;
    for i in 0..96u32 {
        let h = hash2(i.wrapping_mul(17), 91);
        let x = ((h % 1000) as f32 / 1000.0) * sw + px * 0.3;
        let y = (((h >> 10) % 1000) as f32 / 1000.0) * sh * 0.58 + py;
        let s = 1.0 + ((h >> 20) % 3) as f32 * 0.7;
        let twinkle = 0.45 + 0.55 * ((i as f32 * 0.37 + dayness * 8.0).sin());
        let bright = if (h >> 22) % 7 == 0 { 1.15 } else { 1.0 };
        draw.fill_rect(
            Rect::new(x.rem_euclid(sw + 4.0) - 2.0, y, s, s),
            Color::rgba(0.95, 0.97, 1.0, (a * twinkle * bright).min(0.95)),
        );
    }
}

fn paint_planets(draw: &mut DrawList, sw: f32, sh: f32, px: f32, py: f32, dayness: f32) {
    let moon_a = 0.55 + 0.4 * (1.0 - dayness);
    if dayness < 0.85 {
        fill_disc(
            draw,
            sw * 0.78 + px,
            sh * 0.16 + py,
            14.0,
            Color::rgba(0.92, 0.94, 0.88, moon_a * (1.0 - dayness * 0.7)),
        );
    }
    if dayness > 0.15 {
        let sun_a = ((dayness - 0.15) / 0.35).clamp(0.0, 1.0);
        let sx = sw * 0.22 + px;
        let sy = sh * 0.14 + py;
        fill_disc(
            draw,
            sx,
            sy,
            16.0,
            Color::rgba(1.0, 0.86, 0.35, 0.45 * sun_a),
        );
        fill_disc(
            draw,
            sx,
            sy,
            10.0,
            Color::rgba(1.0, 0.96, 0.7, 0.85 * sun_a),
        );
    }
}

fn paint_distant_ridges(
    draw: &mut DrawList,
    sw: f32,
    sh: f32,
    px: f32,
    py: f32,
    dayness: f32,
    ridge: (f32, f32, f32),
) {
    let a = 0.22 + 0.2 * dayness;
    let c = Color::rgba(ridge.0, ridge.1, ridge.2, a);
    let base = sh * 0.58 + py;
    // 窄列锯齿山脊，避免 22–50px 宽矩形柱。
    let mut x = -40.0 + px;
    let mut i = 0u32;
    let mut prev_h = 28.0f32;
    while x < sw + 50.0 {
        let hbits = hash2(i.wrapping_mul(11), 7);
        let w = 2.0 + (hbits % 5) as f32;
        let target = 18.0 + ((hbits >> 4) % 48) as f32;
        let h = prev_h * 0.55 + target * 0.45;
        prev_h = h;
        draw.fill_rect(Rect::new(x, base - h, w, h + sh * 0.22), c);
        x += w;
        i += 1;
    }
}

fn paint_cloud_layers(
    draw: &mut DrawList,
    sw: f32,
    sh: f32,
    px: f32,
    py: f32,
    dayness: f32,
    biome: BiomeId,
) {
    let cloud_a = 0.07 + 0.16 * dayness;
    let cloud = match biome {
        BiomeId::Desert => Color::rgba(0.95, 0.88, 0.75, cloud_a * 0.7),
        BiomeId::Tundra => Color::rgba(0.92, 0.95, 1.0, cloud_a * 1.15),
        _ => Color::rgba(0.95, 0.92, 1.0, cloud_a),
    };
    for (parallax, blobs) in [
        (
            0.45f32,
            [
                (0.12f32, 0.18f32, 78.0f32),
                (0.48, 0.14, 110.0),
                (0.78, 0.22, 70.0),
            ],
        ),
        (
            0.7,
            [(0.28, 0.26, 90.0), (0.62, 0.20, 85.0), (0.90, 0.16, 55.0)],
        ),
    ] {
        for (cx, cy, w) in blobs {
            let x = sw * cx + px * parallax;
            let y = sh * cy + py * (parallax * 0.6);
            draw.fill_rect(Rect::new(x, y, w, 9.0), cloud);
            draw.fill_rect(Rect::new(x + 14.0, y - 7.0, w * 0.55, 8.0), cloud);
            draw.fill_rect(Rect::new(x + w * 0.35, y + 4.0, w * 0.4, 6.0), cloud);
        }
    }
}

fn paint_near_hills(
    draw: &mut DrawList,
    sw: f32,
    sh: f32,
    px: f32,
    py: f32,
    hill_rgb: (f32, f32, f32),
) {
    let hill = Color::rgba(hill_rgb.0, hill_rgb.1, hill_rgb.2, 0.4);
    let hy = sh * 0.74 + py;
    let mut x = -30.0 + px;
    let mut i = 0u32;
    let mut prev_h = 20.0f32;
    while x < sw + 40.0 {
        let hbits = hash2(i.wrapping_mul(23), 41);
        let w = 2.0 + (hbits % 5) as f32;
        let target = 14.0 + ((hbits >> 5) % 28) as f32 + (i as f32 * 0.7).sin().abs() * 10.0;
        let h = prev_h * 0.5 + target * 0.5;
        prev_h = h;
        draw.fill_rect(Rect::new(x, hy - h, w, h + sh * 0.28), hill);
        x += w;
        i += 1;
    }
}

fn mix_rgb(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgb(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    )
}

fn hash2(x: u32, y: u32) -> u32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

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
