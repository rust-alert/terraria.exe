//! 像素精灵绘制（与碰撞盒分离）。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};

use crate::player::{HIT_H, HIT_W, Player};
use crate::world::{DISPLAY_SCALE, TILE, screen_len, screen_of, wrap_delta_x};

const SPRITE_W: usize = 11;
const SPRITE_H: usize = 24;
/// idle / walk×3 / jump / fall
const SPRITE_FRAMES: usize = 6;
const SPRITE_SCALE: f32 = TILE * (DISPLAY_SCALE as f32) * 3.0 / SPRITE_H as f32;

/// 玩家动画态（四态 + 步行子帧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAnim {
    Idle,
    Walk(u8),
    Jump,
    Fall,
}

impl Player {
    /// 由落地 / 速度推导当前动画态。
    pub fn anim_state(&self) -> PlayerAnim {
        if !self.on_ground {
            if self.vy < -12.0 {
                PlayerAnim::Jump
            } else {
                PlayerAnim::Fall
            }
        } else if self.vx.abs() > 1.0 {
            let frame = ((self.walk_phase * 3.0).floor() as i32).rem_euclid(3) as u8;
            PlayerAnim::Walk(frame)
        } else {
            PlayerAnim::Idle
        }
    }
}

/// 正版竖条单帧尺寸。
const VANILLA_CELL_W: u32 = 40;
const VANILLA_CELL_H: u32 = 56;

/// 玩家图集：已合成的横条（每帧一列），或回退像素条。
pub struct PlayerAtlas {
    tex: Option<TextureId>,
    cell_w: u32,
    cell_h: u32,
    columns: u32,
    ready: bool,
}

impl PlayerAtlas {
    pub fn new() -> Self {
        Self {
            tex: None,
            cell_w: SPRITE_W as u32,
            cell_h: SPRITE_H as u32,
            columns: SPRITE_FRAMES as u32,
            ready: false,
        }
    }

    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        if self.try_upload_vanilla(draw, assets) {
            return;
        }
        self.upload_fallback(draw);
    }

    /// 按 `PlayerTextureID` 0..=12 取层，CPU alpha 合成后再上传，避免多层 `tex_rect` 叠坏躯干。
    fn try_upload_vanilla(
        &mut self,
        draw: &mut DrawList,
        assets: &crate::content_boot::ContentAssets,
    ) -> bool {
        // 自下而上：腿 → 身 → 臂 → 头 → 眼。只装 0..=12，排除 Extra / EyeBlink。
        const ORDER: &[u8] = &[10, 11, 12, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2];
        let mut layers: Vec<(u8, crate::xnb::RgbaTexture)> = Vec::new();
        for &id in ORDER {
            let Some(path) = find_player_layer(assets, id) else {
                continue;
            };
            let Ok(tex) = crate::xnb::decode_texture_file(path) else {
                tracing::warn!(id, path = %path.display(), "玩家图层解码失败");
                continue;
            };
            if tex.width != VANILLA_CELL_W || tex.height < VANILLA_CELL_H {
                tracing::warn!(
                    id,
                    w = tex.width,
                    h = tex.height,
                    "玩家图层尺寸不符，已跳过"
                );
                continue;
            }
            layers.push((id, tex));
        }
        let has_head = layers.iter().any(|(id, _)| *id == 0);
        let has_torso = layers.iter().any(|(id, _)| *id == 3 || *id == 6);
        let has_legs = layers.iter().any(|(id, _)| *id == 10 || *id == 12);
        if !has_head || !has_torso || !has_legs {
            tracing::warn!(
                n = layers.len(),
                has_head,
                has_torso,
                has_legs,
                "玩家必要图层不齐，回退像素精灵"
            );
            return false;
        }

        let sheet_h = layers
            .iter()
            .map(|(_, t)| t.height)
            .min()
            .unwrap_or(VANILLA_CELL_H);
        let rows = (sheet_h / VANILLA_CELL_H).max(1);
        let out_w = VANILLA_CELL_W * SPRITE_FRAMES as u32;
        let out_h = VANILLA_CELL_H;
        let mut rgba = vec![0u8; (out_w * out_h * 4) as usize];

        for anim in 0..SPRITE_FRAMES as u32 {
            let src_row = vanilla_body_row(anim as i32, rows);
            let dst_x0 = anim * VANILLA_CELL_W;
            for (id, tex) in &layers {
                let tint = layer_tint(*id);
                blit_layer_frame(&mut rgba, out_w, dst_x0, tex, src_row, tint);
            }
        }

        match draw.create_texture(out_w, out_h, rgba) {
            Ok(id) => {
                self.tex = Some(id);
                self.cell_w = VANILLA_CELL_W;
                self.cell_h = VANILLA_CELL_H;
                self.columns = SPRITE_FRAMES as u32;
                tracing::info!(n = layers.len(), "玩家合成图集已上传");
                true
            }
            Err(e) => {
                tracing::warn!(?e, "玩家合成图集上传失败");
                false
            }
        }
    }

    fn upload_fallback(&mut self, draw: &mut DrawList) {
        let w = (SPRITE_W * SPRITE_FRAMES) as u32;
        let h = SPRITE_H as u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for frame in 0..SPRITE_FRAMES {
            let art = sprite_frame(frame as i32);
            let x0 = (frame * SPRITE_W) as u32;
            for gy in 0..SPRITE_H {
                for gx in 0..SPRITE_W {
                    let idx = art[gy][gx];
                    if idx == 0 {
                        continue;
                    }
                    let c = palette(idx);
                    let i = ((gy as u32 * w + x0 + gx as u32) * 4) as usize;
                    rgba[i] = (c.r * 255.0) as u8;
                    rgba[i + 1] = (c.g * 255.0) as u8;
                    rgba[i + 2] = (c.b * 255.0) as u8;
                    rgba[i + 3] = 255;
                }
            }
        }
        self.cell_w = SPRITE_W as u32;
        self.cell_h = SPRITE_H as u32;
        self.columns = SPRITE_FRAMES as u32;
        if let Ok(id) = draw.create_texture(w, h, rgba) {
            self.tex = Some(id);
        }
    }

    pub(crate) fn paint_icon(&self, draw: &mut DrawList, dest: Rect) {
        let Some(tex) = self.tex else {
            return;
        };
        let uv = self.frame_uv(0);
        draw.tex_rect(tex, dest, uv, Color::rgb(1.0, 1.0, 1.0));
    }

    fn frame_uv(&self, frame: u32) -> Rect {
        let cols = self.columns.max(1);
        let frame = frame % cols;
        let w = 1.0 / cols as f32;
        Rect::new(frame as f32 * w, 0.0, w, 1.0)
    }
}

impl Player {
    pub fn draw(&self, draw: &mut DrawList, cam_x: f32, cam_y: f32, atlas: &PlayerAtlas) {
        let facing_right = self.facing >= 0.0;
        let walk_frame = match self.anim_state() {
            PlayerAnim::Idle => 0,
            PlayerAnim::Walk(f) => 1 + f as i32,
            PlayerAnim::Jump => 4,
            PlayerAnim::Fall => 5,
        };

        let sprite_w = screen_len(atlas.cell_w as f32);
        let sprite_h = screen_len(atlas.cell_h as f32);
        let px = cam_x + wrap_delta_x(cam_x, self.x);
        let origin_x = screen_of(px + HIT_W * 0.5, cam_x) - sprite_w * 0.5;
        let origin_y = screen_of(self.y + HIT_H, cam_y) - sprite_h;
        let dest = Rect::new(origin_x, origin_y, sprite_w, sprite_h);

        paint_ground_shadow(draw, self, cam_x, cam_y, sprite_w);

        // 无敌帧闪烁：跳过若干帧制造受击反馈，不改图集。
        if self.iframes > 0.0 {
            let blink = ((self.iframes * 18.0) as i32) & 1 == 0;
            if blink {
                return;
            }
        }
        let hurt_tint = if self.iframes > 0.35 {
            Color::rgb(1.0, 0.55, 0.55)
        } else {
            Color::rgb(1.0, 1.0, 1.0)
        };

        if let Some(tex) = atlas.tex {
            let frame = walk_frame.max(0) as u32;
            let mut uv = atlas.frame_uv(frame);
            if !facing_right {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            draw.tex_rect(tex, dest, uv, hurt_tint);
            return;
        }

        let art = sprite_frame(walk_frame);
        for gy in 0..SPRITE_H {
            for gx in 0..SPRITE_W {
                let src_x = if facing_right { gx } else { SPRITE_W - 1 - gx };
                let idx = art[gy][src_x];
                if idx == 0 {
                    continue;
                }
                let mut color = palette(idx);
                if self.iframes > 0.35 {
                    color = Color::rgba(
                        (color.r * 0.55 + 0.45).min(1.0),
                        color.g * 0.55,
                        color.b * 0.55,
                        color.a,
                    );
                }
                let sx = origin_x + gx as f32 * SPRITE_SCALE;
                let sy = origin_y + gy as f32 * SPRITE_SCALE;
                draw.fill_rect(Rect::new(sx, sy, SPRITE_SCALE, SPRITE_SCALE), color);
            }
        }
    }
}

/// 触地椭圆阴影：落地时更实，腾空随高度衰减。
fn paint_ground_shadow(
    draw: &mut DrawList,
    player: &Player,
    cam_x: f32,
    cam_y: f32,
    sprite_w: f32,
) {
    let px = cam_x + wrap_delta_x(cam_x, player.x);
    let foot_x = screen_of(px + HIT_W * 0.5, cam_x);
    let foot_y = screen_of(player.y + HIT_H, cam_y);

    let height = if player.on_ground {
        0.0
    } else {
        // 近似离地：用竖直速度与未着地状态压暗
        (player.vy.abs() * 0.04 + 4.0).min(28.0)
    };
    let fade = (1.0 - height / 28.0).clamp(0.15, 1.0);
    let strength = if player.on_ground { 0.55 } else { 0.22 * fade };
    let rx = sprite_w * (0.44 + 0.06 * fade);
    let ry = if player.on_ground {
        3.6
    } else {
        2.4 + 1.6 * fade
    };

    // 三层软椭圆（逐行填充）。
    for layer in 0..3 {
        let t = layer as f32 / 2.0;
        let a = strength * (1.0 - t * 0.45);
        let w = rx * (1.0 + t * 0.35);
        let h = ry * (1.0 + t * 0.55);
        let color = Color::rgba(0.02, 0.02, 0.05, a);
        let y0 = (foot_y - h).floor() as i32;
        let y1 = (foot_y + h * 0.35).ceil() as i32;
        for y in y0..=y1 {
            let dy = (y as f32 + 0.5 - foot_y) / h.max(0.5);
            let inner = 1.0 - dy * dy;
            if inner <= 0.0 {
                continue;
            }
            let half = w * inner.sqrt();
            draw.fill_rect(Rect::new(foot_x - half, y as f32, half * 2.0, 1.0), color);
        }
    }
}

fn palette(i: u8) -> Color {
    match i {
        1 => Color::rgb(0.12, 0.10, 0.14), // 描边 / 鞋
        2 => Color::rgb(0.35, 0.22, 0.12), // 头发
        3 => Color::rgb(0.93, 0.78, 0.62), // 肤色
        4 => Color::rgb(0.22, 0.45, 0.72), // 上衣
        5 => Color::rgb(0.28, 0.32, 0.42), // 裤
        6 => Color::rgb(0.95, 0.95, 0.98), // 眼白
        7 => Color::rgb(0.10, 0.12, 0.18), // 瞳孔
        8 => Color::rgb(0.55, 0.18, 0.18), // 袖口 / 点缀
        _ => Color::rgb(1.0, 0.0, 1.0),
    }
}

/// 站立 / 走 / 跳 的像素模板（朝右）。行优先，上→下。
fn sprite_frame(frame: i32) -> [[u8; SPRITE_W]; SPRITE_H] {
    // 公共上身
    let mut m = [[0u8; SPRITE_W]; SPRITE_H];
    // 头发
    put(
        &mut m,
        &[
            (3, 0, 2),
            (4, 0, 2),
            (5, 0, 2),
            (6, 0, 2),
            (7, 0, 2),
            (2, 1, 2),
            (3, 1, 2),
            (4, 1, 2),
            (5, 1, 2),
            (6, 1, 2),
            (7, 1, 2),
            (8, 1, 2),
        ],
    );
    // 头 + 描边
    put(
        &mut m,
        &[
            (2, 2, 1),
            (3, 2, 3),
            (4, 2, 3),
            (5, 2, 3),
            (6, 2, 3),
            (7, 2, 3),
            (8, 2, 1),
            (2, 3, 1),
            (3, 3, 3),
            (4, 3, 3),
            (5, 3, 3),
            (6, 3, 3),
            (7, 3, 3),
            (8, 3, 1),
            (2, 4, 1),
            (3, 4, 3),
            (4, 4, 3),
            (5, 4, 3),
            (6, 4, 3),
            (7, 4, 3),
            (8, 4, 1),
            (3, 5, 1),
            (4, 5, 3),
            (5, 5, 3),
            (6, 5, 3),
            (7, 5, 1),
        ],
    );
    // 眼睛（朝右）
    put(&mut m, &[(6, 3, 6), (7, 3, 7)]);
    // 身子
    put(
        &mut m,
        &[
            (3, 6, 1),
            (4, 6, 4),
            (5, 6, 4),
            (6, 6, 4),
            (7, 6, 1),
            (2, 7, 1),
            (3, 7, 4),
            (4, 7, 4),
            (5, 7, 4),
            (6, 7, 4),
            (7, 7, 4),
            (8, 7, 1),
            (2, 8, 1),
            (3, 8, 4),
            (4, 8, 4),
            (5, 8, 4),
            (6, 8, 4),
            (7, 8, 4),
            (8, 8, 8),
            (3, 9, 1),
            (4, 9, 4),
            (5, 9, 4),
            (6, 9, 4),
            (7, 9, 1),
            (3, 10, 1),
            (4, 10, 4),
            (5, 10, 4),
            (6, 10, 4),
            (7, 10, 1),
            (3, 11, 1),
            (4, 11, 4),
            (5, 11, 4),
            (6, 11, 4),
            (7, 11, 1),
        ],
    );
    // 手臂（静止贴身；走/跳/落略摆）
    let arm = match frame {
        1 | 4 => vec![(8, 7, 3), (9, 8, 3), (9, 9, 3)],
        3 | 5 => vec![(8, 7, 3), (8, 8, 3), (7, 9, 3)],
        2 => vec![(8, 7, 3), (9, 7, 3), (9, 8, 3)],
        _ => vec![(8, 7, 3), (8, 8, 3), (8, 9, 3)],
    };
    put(&mut m, &arm);

    // 腿 + 鞋
    let legs: &[(usize, usize, u8)] = match frame {
        1 => &[
            (3, 12, 5),
            (4, 12, 5),
            (6, 12, 5),
            (7, 12, 5),
            (3, 13, 5),
            (4, 13, 5),
            (6, 13, 5),
            (7, 13, 5),
            (2, 14, 5),
            (3, 14, 5),
            (7, 14, 5),
            (8, 14, 5),
            (2, 15, 5),
            (3, 15, 5),
            (7, 15, 5),
            (8, 15, 5),
            (2, 16, 1),
            (3, 16, 1),
            (7, 16, 1),
            (8, 16, 1),
            (2, 17, 1),
            (3, 17, 1),
            (7, 17, 1),
            (8, 17, 1),
        ],
        2 => &[
            (4, 12, 5),
            (5, 12, 5),
            (6, 12, 5),
            (3, 13, 5),
            (4, 13, 5),
            (6, 13, 5),
            (7, 13, 5),
            (3, 14, 5),
            (4, 14, 5),
            (6, 14, 5),
            (7, 14, 5),
            (3, 15, 5),
            (7, 15, 5),
            (2, 16, 1),
            (3, 16, 1),
            (7, 16, 1),
            (8, 16, 1),
            (2, 17, 1),
            (3, 17, 1),
            (7, 17, 1),
            (8, 17, 1),
        ],
        3 => &[
            (3, 12, 5),
            (4, 12, 5),
            (6, 12, 5),
            (7, 12, 5),
            (3, 13, 5),
            (4, 13, 5),
            (6, 13, 5),
            (7, 13, 5),
            (3, 14, 5),
            (4, 14, 5),
            (6, 14, 5),
            (7, 14, 5),
            (4, 15, 5),
            (5, 15, 5),
            (6, 15, 5),
            (4, 16, 1),
            (5, 16, 1),
            (6, 16, 1),
            (4, 17, 1),
            (5, 17, 1),
            (6, 17, 1),
        ],
        4 => &[
            // 跳：腿收起、略前倾
            (4, 12, 5),
            (5, 12, 5),
            (6, 12, 5),
            (3, 13, 5),
            (4, 13, 5),
            (6, 13, 5),
            (7, 13, 5),
            (3, 14, 5),
            (7, 14, 5),
            (2, 15, 1),
            (3, 15, 1),
            (7, 15, 1),
            (8, 15, 1),
        ],
        5 => &[
            // 落：腿伸展、略后仰
            (4, 12, 5),
            (5, 12, 5),
            (6, 12, 5),
            (3, 13, 5),
            (4, 13, 5),
            (6, 13, 5),
            (7, 13, 5),
            (2, 14, 5),
            (3, 14, 5),
            (7, 14, 5),
            (8, 14, 5),
            (2, 15, 5),
            (3, 15, 5),
            (7, 15, 5),
            (8, 15, 5),
            (1, 16, 1),
            (2, 16, 1),
            (8, 16, 1),
            (9, 16, 1),
            (1, 17, 1),
            (2, 17, 1),
            (8, 17, 1),
            (9, 17, 1),
        ],
        _ => &[
            // 站立
            (4, 12, 5),
            (5, 12, 5),
            (6, 12, 5),
            (4, 13, 5),
            (5, 13, 5),
            (6, 13, 5),
            (4, 14, 5),
            (6, 14, 5),
            (4, 15, 5),
            (6, 15, 5),
            (4, 16, 1),
            (6, 16, 1),
            (3, 17, 1),
            (4, 17, 1),
            (6, 17, 1),
            (7, 17, 1),
            (3, 18, 1),
            (4, 18, 1),
            (6, 18, 1),
            (7, 18, 1),
        ],
    };
    put(&mut m, legs);
    m
}

fn put(m: &mut [[u8; SPRITE_W]; SPRITE_H], cells: &[(usize, usize, u8)]) {
    for &(x, y, c) in cells {
        if x < SPRITE_W && y < SPRITE_H {
            m[y][x] = c;
        }
    }
}

fn find_player_layer(
    assets: &crate::content_boot::ContentAssets,
    id: u8,
) -> Option<&std::path::Path> {
    let needle = format!("Player_0_{id}.xnb");
    assets
        .player_sheets
        .iter()
        .find(|p| p.file_name().and_then(|s| s.to_str()) == Some(needle.as_str()))
        .map(std::path::PathBuf::as_path)
}

/// 把玩法动画帧映射到正版竖条行号（身/头共用）。
fn vanilla_body_row(anim: i32, rows: u32) -> u32 {
    let row = match anim {
        0 => 0u32,
        1 => 6,
        2 => 7,
        3 => 8,
        4 | 5 => 5,
        _ => 0,
    };
    row.min(rows.saturating_sub(1))
}

/// 默认角色染色。皮肤层贴图本身已带肤色，乘白色；衣物层为灰度，乘衣服色。
fn layer_tint(id: u8) -> Color {
    match id {
        // Undershirt / ArmUndershirt
        4 | 8 => Color::rgb(160.0 / 255.0, 180.0 / 255.0, 215.0 / 255.0),
        // Shirt
        6 => Color::rgb(175.0 / 255.0, 75.0 / 255.0, 75.0 / 255.0),
        // Pants
        11 => Color::rgb(255.0 / 255.0, 230.0 / 255.0, 175.0 / 255.0),
        // Shoes
        12 => Color::rgb(160.0 / 255.0, 105.0 / 255.0, 60.0 / 255.0),
        _ => Color::rgb(1.0, 1.0, 1.0),
    }
}

/// 把源竖条第 `src_row` 帧 tint 后 alpha-over 到合成横条。
fn blit_layer_frame(
    dst: &mut [u8],
    dst_w: u32,
    dst_x0: u32,
    src: &crate::xnb::RgbaTexture,
    src_row: u32,
    tint: Color,
) {
    let y0 = src_row * VANILLA_CELL_H;
    if y0 + VANILLA_CELL_H > src.height {
        return;
    }
    for ly in 0..VANILLA_CELL_H {
        for lx in 0..VANILLA_CELL_W {
            let si = ((y0 + ly) * src.width + lx) * 4;
            let si = si as usize;
            if si + 3 >= src.rgba.len() {
                continue;
            }
            let sa = src.rgba[si + 3] as f32 / 255.0;
            if sa < 1.0 / 255.0 {
                continue;
            }
            let sr = (src.rgba[si] as f32 / 255.0) * tint.r;
            let sg = (src.rgba[si + 1] as f32 / 255.0) * tint.g;
            let sb = (src.rgba[si + 2] as f32 / 255.0) * tint.b;
            let di = ((ly * dst_w + dst_x0 + lx) * 4) as usize;
            if di + 3 >= dst.len() {
                continue;
            }
            let da = dst[di + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a < 1.0 / 255.0 {
                dst[di] = 0;
                dst[di + 1] = 0;
                dst[di + 2] = 0;
                dst[di + 3] = 0;
                continue;
            }
            let dr = dst[di] as f32 / 255.0;
            let dg = dst[di + 1] as f32 / 255.0;
            let db = dst[di + 2] as f32 / 255.0;
            let or = (sr * sa + dr * da * (1.0 - sa)) / out_a;
            let og = (sg * sa + dg * da * (1.0 - sa)) / out_a;
            let ob = (sb * sa + db * da * (1.0 - sa)) / out_a;
            dst[di] = (or * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[di + 1] = (og * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[di + 2] = (ob * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[di + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_row_mapping_stays_in_sheet() {
        assert_eq!(vanilla_body_row(0, 20), 0);
        assert_eq!(vanilla_body_row(1, 20), 6);
        assert_eq!(vanilla_body_row(5, 20), 5);
        assert_eq!(vanilla_body_row(3, 4), 3);
    }

    #[test]
    fn alpha_over_keeps_lower_layer_body() {
        let mut dst = vec![0u8; (VANILLA_CELL_W * VANILLA_CELL_H * 4) as usize];
        // 下层：整帧不透明红
        for i in 0..(VANILLA_CELL_W * VANILLA_CELL_H) as usize {
            let o = i * 4;
            dst[o] = 200;
            dst[o + 1] = 40;
            dst[o + 2] = 40;
            dst[o + 3] = 255;
        }
        // 上层：仅顶部一行不透明白（模拟头），其余透明
        let mut src_rgba = vec![0u8; (VANILLA_CELL_W * VANILLA_CELL_H * 4) as usize];
        for x in 0..VANILLA_CELL_W {
            let o = (x * 4) as usize;
            src_rgba[o] = 255;
            src_rgba[o + 1] = 255;
            src_rgba[o + 2] = 255;
            src_rgba[o + 3] = 255;
        }
        let src = crate::xnb::RgbaTexture {
            name: "t".into(),
            width: VANILLA_CELL_W,
            height: VANILLA_CELL_H,
            rgba: src_rgba,
        };
        blit_layer_frame(
            &mut dst,
            VANILLA_CELL_W,
            0,
            &src,
            0,
            Color::rgb(1.0, 1.0, 1.0),
        );
        // 头顶被盖成白
        assert_eq!(dst[0], 255);
        assert_eq!(dst[3], 255);
        // 中部躯干仍是红
        let mid = ((VANILLA_CELL_H / 2) * VANILLA_CELL_W * 4) as usize;
        assert_eq!(dst[mid], 200);
        assert_eq!(dst[mid + 3], 255);
    }
}
