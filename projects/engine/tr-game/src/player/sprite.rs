//! 像素精灵绘制（与碰撞盒分离）。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};

use crate::player::{HIT_H, HIT_W, Player};
use crate::world::{TILE, wrap_delta_x};

const SPRITE_W: usize = 11;
const SPRITE_H: usize = 24;
/// idle / walk×3 / jump / fall
const SPRITE_FRAMES: usize = 6;
const SPRITE_SCALE: f32 = TILE * 3.0 / SPRITE_H as f32;

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

struct PlayerLayer {
    tex: TextureId,
    src_h: u32,
}

/// 玩家图层。`columns == 1` 时按 `cell_h` 从贴图高度切竖直帧，否则按列切水平条。
pub struct PlayerAtlas {
    layers: Vec<PlayerLayer>,
    cell_w: u32,
    cell_h: u32,
    columns: u32,
    ready: bool,
}

impl PlayerAtlas {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
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
        let mut paths: Vec<&std::path::Path> = assets
            .player_sheets
            .iter()
            .map(std::path::PathBuf::as_path)
            .filter(|p| {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                name.starts_with("Player_0_") || name == "Player_Hair_1.xnb"
            })
            .collect();
        paths.sort_by_key(|p| player_layer_order(p));
        for path in paths {
            let Ok(tex) = crate::xnb::decode_texture_file(path) else {
                continue;
            };
            if tex.width != 40 || tex.height < 56 {
                continue;
            }
            match draw.create_texture(tex.width, tex.height, tex.rgba) {
                Ok(id) => {
                    self.cell_w = 40;
                    self.cell_h = 56;
                    self.columns = 1;
                    self.layers.push(PlayerLayer {
                        tex: id,
                        src_h: tex.height,
                    });
                }
                Err(e) => tracing::warn!(?e, "玩家图层上传失败"),
            }
        }
        if self.layers.is_empty() {
            self.upload_fallback(draw);
        } else {
            tracing::info!(n = self.layers.len(), "玩家图层已上传");
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
            self.layers.push(PlayerLayer { tex: id, src_h: h });
        }
    }

    pub(crate) fn paint_icon(&self, draw: &mut DrawList, dest: Rect) {
        if self.layers.is_empty() {
            return;
        }
        for layer in &self.layers {
            let uv = self.frame_uv(0, layer.src_h);
            draw.tex_rect(layer.tex, dest, uv, Color::rgb(1.0, 1.0, 1.0));
        }
    }

    fn frame_uv(&self, frame: u32, src_h: u32) -> Rect {
        if self.columns <= 1 {
            let cell = self.cell_h.max(1);
            let rows = (src_h / cell).max(1);
            let frame = frame % rows;
            let h = cell as f32 / src_h.max(1) as f32;
            Rect::new(0.0, frame as f32 * h, 1.0, h)
        } else {
            let cols = self.columns.max(1);
            let frame = frame % cols;
            let w = 1.0 / cols as f32;
            Rect::new(frame as f32 * w, 0.0, w, 1.0)
        }
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

        let scale = TILE / 16.0;
        let sprite_w = atlas.cell_w as f32 * scale;
        let sprite_h = atlas.cell_h as f32 * scale;
        // 回环：把碰撞盒左缘解到相机附近的周期像再画
        let px = cam_x + wrap_delta_x(cam_x, self.x);
        let origin_x = px + HIT_W * 0.5 - sprite_w * 0.5 - cam_x;
        let origin_y = self.y + HIT_H - sprite_h - cam_y;
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

        if !atlas.layers.is_empty() {
            let frame = walk_frame.max(0) as u32;
            for layer in &atlas.layers {
                let mut uv = atlas.frame_uv(frame, layer.src_h);
                if !facing_right {
                    uv.x += uv.w;
                    uv.w = -uv.w;
                }
                draw.tex_rect(layer.tex, dest, uv, hurt_tint);
            }
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
    let foot_x = px + HIT_W * 0.5 - cam_x;
    let foot_y = player.y + HIT_H - cam_y;

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

fn player_layer_order(path: &std::path::Path) -> u32 {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.starts_with("Player_Hair_") {
        return 10_000;
    }
    name.strip_prefix("Player_0_")
        .and_then(|s| s.strip_suffix(".xnb"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(9_000)
}
