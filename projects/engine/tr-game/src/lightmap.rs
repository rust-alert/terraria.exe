//! 视口光照图：天空柱 + 人造光源 BFS，RGB 通道各自传播（彩色光照）。

use std::collections::VecDeque;
use tr_core::BlockId;

use crate::world::{WORLD_H, World, wrap_tx};

/// 单格光照采样：乘到材质色上（`shade`）。
#[derive(Debug, Clone, Copy)]
pub struct LightSample {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl LightSample {
    pub fn intensity(self) -> f32 {
        self.r.max(self.g).max(self.b)
    }

    /// 各通道不低于 `floor`（接触阴影等需要保底）。
    pub fn max_with(self, floor: f32) -> Self {
        Self {
            r: self.r.max(floor),
            g: self.g.max(floor),
            b: self.b.max(floor),
        }
    }
}

/// 视口局部彩色光照缓冲（含边距）。
pub struct LightMap {
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
}

impl LightMap {
    pub fn sample(&self, tx: i32, ty: i32) -> LightSample {
        if ty < self.y0 || ty >= self.y0 + self.h {
            return LightSample {
                r: 0.05,
                g: 0.05,
                b: 0.06,
            };
        }
        let x = wrap_tx(tx);
        let lx = x - self.x0;
        if lx < 0 || lx >= self.w {
            return LightSample {
                r: 0.05,
                g: 0.05,
                b: 0.06,
            };
        }
        let i = self.idx(lx, ty - self.y0);
        LightSample {
            r: self.r[i].clamp(0.04, 1.35),
            g: self.g[i].clamp(0.04, 1.35),
            b: self.b[i].clamp(0.04, 1.35),
        }
    }

    fn idx(&self, lx: i32, ly: i32) -> usize {
        (ly * self.w + lx) as usize
    }

    fn bump_rgb(&mut self, i: usize, nr: f32, ng: f32, nb: f32) -> bool {
        let mut changed = false;
        if nr > self.r[i] + 0.008 {
            self.r[i] = nr;
            changed = true;
        }
        if ng > self.g[i] + 0.008 {
            self.g[i] = ng;
            changed = true;
        }
        if nb > self.b[i] + 0.008 {
            self.b[i] = nb;
            changed = true;
        }
        changed
    }

    /// 在 `[x0,x1]×[y0,y1]` 外包一圈 `pad` 后重建。
    /// `dayness` 影响天光色温（昼偏暖白，夜偏冷蓝）。
    pub fn build(
        world: &World,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        sky_amb: f32,
        dayness: f32,
    ) -> Self {
        let pad = 14;
        let bx0 = x0 - pad;
        let by0 = (y0 - pad).max(0);
        let bx1 = x1 + pad;
        let by1 = (y1 + pad).min(WORLD_H - 1);
        let w = bx1 - bx0 + 1;
        let h = by1 - by0 + 1;
        let n = (w * h) as usize;
        let mut map = Self {
            x0: bx0,
            y0: by0,
            w,
            h,
            r: vec![0.04f32; n],
            g: vec![0.04f32; n],
            b: vec![0.05f32; n],
        };

        let (sky_r, sky_g, sky_b) = sky_tint(dayness);
        let mut queue: VecDeque<(i32, i32)> = VecDeque::with_capacity(n / 2);

        // 1) 天空柱：自上而下，遇挡光固体切断。
        for lx in 0..w {
            let tx = wrap_tx(bx0 + lx);
            let mut sky = sky_amb;
            for ly in 0..h {
                let ty = by0 + ly;
                let id = world.get(tx, ty);
                let i = map.idx(lx, ly);
                if id.blocks_light() {
                    let s = sky * 0.22;
                    let _ = map.bump_rgb(i, sky_r * s, sky_g * s, sky_b * s);
                    sky = 0.04;
                } else {
                    let _ = map.bump_rgb(i, sky_r * sky, sky_g * sky, sky_b * sky);
                    sky = (sky * 0.985).max(0.04);
                    queue.push_back((lx, ly));
                }
            }
        }

        // 2) 人造光源注入（彩色）。
        for ly in 0..h {
            for lx in 0..w {
                let tx = wrap_tx(bx0 + lx);
                let ty = by0 + ly;
                let id = world.get(tx, ty);
                if !id.emits_light() {
                    continue;
                }
                let (strength, er, eg, eb) = emitter_rgb(id);
                let i = map.idx(lx, ly);
                if map.bump_rgb(i, er * strength, eg * strength, eb * strength) {
                    queue.push_back((lx, ly));
                }
            }
        }

        // 3) BFS：RGB 同衰减传播。
        const EMPTY_FALL: f32 = 0.085;
        const SOLID_FALL: f32 = 0.38;
        while let Some((lx, ly)) = queue.pop_front() {
            let i = map.idx(lx, ly);
            let cr = map.r[i];
            let cg = map.g[i];
            let cb = map.b[i];
            if cr.max(cg).max(cb) <= 0.06 {
                continue;
            }
            let tx = wrap_tx(bx0 + lx);
            let ty = by0 + ly;
            let cur_block = world.get(tx, ty);
            let from_solid = cur_block.blocks_light();

            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nlx = lx + dx;
                let nly = ly + dy;
                if nlx < 0 || nlx >= w || nly < 0 || nly >= h {
                    continue;
                }
                let ntx = wrap_tx(bx0 + nlx);
                let nty = by0 + nly;
                let nid = world.get(ntx, nty);
                let to_solid = nid.blocks_light();
                let mut fall = if to_solid { SOLID_FALL } else { EMPTY_FALL };
                if from_solid {
                    fall += 0.12;
                }
                if matches!(nid, BlockId::LEAF | BlockId::WATER) {
                    fall += 0.04;
                }
                let ni = map.idx(nlx, nly);
                if map.bump_rgb(ni, cr - fall, cg - fall, cb - fall) {
                    queue.push_back((nlx, nly));
                }
            }
        }

        // 深度保底：极深洞穴保留微弱冷光，避免纯黑。
        for i in 0..n {
            map.r[i] = map.r[i].max(0.04);
            map.g[i] = map.g[i].max(0.045);
            map.b[i] = map.b[i].max(0.055);
        }

        map
    }
}

fn sky_tint(dayness: f32) -> (f32, f32, f32) {
    let t = dayness.clamp(0.0, 1.0);
    // 昼：略暖白；夜：冷蓝紫。
    let day = (1.0, 0.97, 0.92);
    let night = (0.32, 0.40, 0.78);
    (
        night.0 + (day.0 - night.0) * t,
        night.1 + (day.1 - night.1) * t,
        night.2 + (day.2 - night.2) * t,
    )
}

fn emitter_rgb(id: BlockId) -> (f32, f32, f32, f32) {
    use crate::palette::{GLOW_DEFAULT, GLOW_FURNACE, GLOW_TORCH};
    match id {
        BlockId::TORCH => (1.0, GLOW_TORCH.r, GLOW_TORCH.g, GLOW_TORCH.b),
        BlockId::FURNACE => (0.92, GLOW_FURNACE.r, GLOW_FURNACE.g, GLOW_FURNACE.b),
        _ => (0.75, GLOW_DEFAULT.r, GLOW_DEFAULT.g, GLOW_DEFAULT.b),
    }
}
