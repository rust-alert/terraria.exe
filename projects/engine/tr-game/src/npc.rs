//! 城镇 NPC（向导、商人等）。贴图用 `NPC_N.xnb`。

use spark_core::{Color, Rect};
use spark_renderer::{DrawList, TextureId};

use crate::housing::HouseSlot;
use crate::world::{TILE, World, screen_len, screen_of, wrap_tx};

/// 城镇 NPC 种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TownKind {
    /// 向导（`NPC_22`）。
    Guide,
    /// 商人（`NPC_17`）。
    Merchant,
}

impl TownKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Guide => "向导",
            Self::Merchant => "商人",
        }
    }

    pub fn npc_file(self) -> u32 {
        let id = match self {
            Self::Guide => tr_core::NpcId::GUIDE,
            Self::Merchant => tr_core::NpcId::MERCHANT,
        };
        id.texture_file().unwrap_or(id.0)
    }

    pub fn lines(self) -> &'static [&'static str] {
        match self {
            Self::Guide => &[
                "欢迎来到这个世界。砍树、造工作台，先把住处搭起来。",
                "工作台把木材做成木墙。左键铺墙，封闭房间再放火把和床。",
                "站在房间里按 H 查询是否合格。合格后由住房分配入住。",
                "第二间空房会引来商人。",
            ],
            Self::Merchant => &[
                "有空房我就来。靠近我按 E 打开商店。",
                "铜币不够？去打史莱姆。",
                "火把、绳索、木墙这里都有。",
            ],
        }
    }
}

/// 一个城镇 NPC。
#[derive(Debug, Clone)]
pub struct TownNpc {
    pub kind: TownKind,
    pub x: f32,
    pub y: f32,
    pub facing: f32,
    pub line_i: usize,
    /// 已入住房屋在 `houses` 中的下标。
    pub home: Option<usize>,
}

impl TownNpc {
    pub fn at_surface(world: &World, kind: TownKind, tx: i32) -> Self {
        let tx = wrap_tx(tx);
        let sh = world.surface_at(tx);
        let hit_h = TILE * (42.0 / 16.0);
        let hit_w = TILE * (20.0 / 16.0);
        let feet_y = sh as f32 * TILE;
        Self {
            kind,
            x: tx as f32 * TILE + (TILE - hit_w) * 0.5,
            y: feet_y - hit_h,
            facing: -1.0,
            line_i: 0,
            home: None,
        }
    }

    pub fn guide_at(world: &World, tx: i32) -> Self {
        Self::at_surface(world, TownKind::Guide, tx)
    }

    pub fn hitbox(&self) -> (f32, f32, f32, f32) {
        let w = TILE * (20.0 / 16.0);
        let h = TILE * (42.0 / 16.0);
        (self.x, self.y, w, h)
    }

    pub fn near_player(&self, px: f32, py: f32, pw: f32, ph: f32) -> bool {
        let (nx, ny, nw, nh) = self.hitbox();
        let cx = px + pw * 0.5;
        let cy = py + ph * 0.5;
        let ncx = nx + nw * 0.5;
        let ncy = ny + nh * 0.5;
        let dx = (cx - ncx).abs() / TILE;
        let dy = (cy - ncy).abs() / TILE;
        dx < 3.5 && dy < 3.5
    }

    /// 推进一句对话，返回当前台词。
    pub fn talk_next(&mut self) -> &'static str {
        let lines = self.kind.lines();
        let line = lines[self.line_i % lines.len()];
        self.line_i = (self.line_i + 1) % lines.len();
        line
    }

    /// 搬进房屋站立点。
    pub fn move_into(&mut self, house_i: usize, stand: (i32, i32)) {
        let hit_h = TILE * (42.0 / 16.0);
        let hit_w = TILE * (20.0 / 16.0);
        let (tx, ty) = stand;
        self.x = tx as f32 * TILE + (TILE - hit_w) * 0.5;
        self.y = (ty + 1) as f32 * TILE - hit_h;
        self.home = Some(house_i);
    }

    pub fn draw(&self, draw: &mut DrawList, cam_x: f32, cam_y: f32, atlas: &NpcAtlas) {
        let (w, h) = (
            screen_len(TILE * (20.0 / 16.0)),
            screen_len(TILE * (42.0 / 16.0)),
        );
        let sx = screen_of(self.x, cam_x);
        let sy = screen_of(self.y, cam_y);
        if let Some(view) = atlas.view(self.kind) {
            let mut uv = view.uv;
            if self.facing < 0.0 {
                uv.x += uv.w;
                uv.w = -uv.w;
            }
            let (cw, ch) = atlas.town_cell(self.kind);
            let sprite_w = screen_len(cw as f32);
            let sprite_h = screen_len(ch as f32);
            let ox = sx + w * 0.5 - sprite_w * 0.5;
            let oy = sy + h - sprite_h;
            draw.tex_rect(
                view.tex,
                Rect::new(ox, oy, sprite_w, sprite_h),
                uv,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
        } else {
            let body = match self.kind {
                TownKind::Guide => Color::rgb(0.55, 0.45, 0.35),
                TownKind::Merchant => Color::rgb(0.35, 0.45, 0.55),
            };
            draw.fill_rect(Rect::new(sx, sy, w, h), body);
            draw.fill_rect(
                Rect::new(sx + 4.0, sy + 4.0, w - 8.0, 10.0),
                Color::rgb(0.95, 0.85, 0.7),
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NpcView {
    pub tex: TextureId,
    pub uv: Rect,
}

/// 城镇 NPC 图集（按种类缓存第一站立帧）+ 敌怪僵尸帧。
#[derive(Debug, Default)]
pub struct NpcAtlas {
    guide: Option<(TextureId, Rect)>,
    merchant: Option<(TextureId, Rect)>,
    guide_cell: (u32, u32),
    merchant_cell: (u32, u32),
    zombie: Option<(TextureId, Rect)>,
    demon_eye: Option<(TextureId, Rect)>,
    demon_cell: (u32, u32),
    cell_w: u32,
    cell_h: u32,
    ready: bool,
}

impl NpcAtlas {
    pub fn new() -> Self {
        Self {
            guide: None,
            merchant: None,
            guide_cell: (40, 56),
            merchant_cell: (40, 56),
            zombie: None,
            demon_eye: None,
            demon_cell: (32, 22),
            cell_w: 40,
            cell_h: 56,
            ready: false,
        }
    }

    pub fn ensure(&mut self, draw: &mut DrawList, assets: &crate::content_boot::ContentAssets) {
        if self.ready {
            return;
        }
        self.ready = true;
        for kind in [TownKind::Guide, TownKind::Merchant] {
            let file = kind.npc_file();
            let Some(path) = assets.npc_sheets.get(&file) else {
                tracing::warn!(file, "缺少城镇 NPC 图集");
                continue;
            };
            let Ok(tex) = crate::xnb::decode_texture_file(path) else {
                continue;
            };
            if tex.width == 0 || tex.height < 40 {
                continue;
            }
            let id = match kind {
                TownKind::Guide => tr_core::NpcId::GUIDE,
                TownKind::Merchant => tr_core::NpcId::MERCHANT,
            };
            let frames = crate::sheets::npc_frame_count(id).max(1);
            let cell_h = (tex.height / frames).max(1);
            let uv = Rect::new(0.0, 0.0, 1.0, cell_h as f32 / tex.height as f32);
            match draw.create_texture(tex.width, tex.height, tex.rgba) {
                Ok(gpu) => {
                    let cell = (tex.width, cell_h);
                    match kind {
                        TownKind::Guide => {
                            self.guide_cell = cell;
                            self.guide = Some((gpu, uv));
                        }
                        TownKind::Merchant => {
                            self.merchant_cell = cell;
                            self.merchant = Some((gpu, uv));
                        }
                    }
                }
                Err(e) => tracing::warn!(?e, file, "NPC 纹理上传失败"),
            }
        }
        if let Some(file) = crate::sheets::npc_file(tr_core::NpcId::ZOMBIE) {
            if let Some(path) = assets.npc_sheets.get(&file) {
                if let Ok(tex) = crate::xnb::decode_texture_file(path) {
                    if tex.width > 0 && tex.height >= 40 {
                        let frames = crate::sheets::npc_frame_count(tr_core::NpcId::ZOMBIE);
                        let cell_h = (tex.height / frames.max(1)).max(1);
                        let uv = Rect::new(0.0, 0.0, 1.0, cell_h as f32 / tex.height as f32);
                        match draw.create_texture(tex.width, tex.height, tex.rgba) {
                            Ok(id) => {
                                self.cell_w = tex.width;
                                self.cell_h = cell_h;
                                self.zombie = Some((id, uv));
                            }
                            Err(e) => tracing::warn!(?e, "僵尸纹理上传失败"),
                        }
                    }
                }
            }
        }
        if let Some(file) = crate::sheets::npc_file(tr_core::NpcId::DEMON_EYE) {
            if let Some(path) = assets.npc_sheets.get(&file) {
                if let Ok(tex) = crate::xnb::decode_texture_file(path) {
                    if tex.width > 0 && tex.height > 0 {
                        let frames = crate::sheets::npc_frame_count(tr_core::NpcId::DEMON_EYE);
                        let cell_h = (tex.height / frames.max(1)).max(1);
                        let uv = Rect::new(0.0, 0.0, 1.0, cell_h as f32 / tex.height as f32);
                        match draw.create_texture(tex.width, tex.height, tex.rgba) {
                            Ok(id) => {
                                self.demon_cell = (tex.width, cell_h);
                                self.demon_eye = Some((id, uv));
                            }
                            Err(e) => tracing::warn!(?e, "恶魔眼纹理上传失败"),
                        }
                    }
                }
            }
        }
        tracing::info!(
            guide = self.guide.is_some(),
            merchant = self.merchant.is_some(),
            zombie = self.zombie.is_some(),
            demon_eye = self.demon_eye.is_some(),
            "城镇 / 敌怪 NPC 图集已上传"
        );
    }

    pub fn town_cell(&self, kind: TownKind) -> (u32, u32) {
        match kind {
            TownKind::Guide => self.guide_cell,
            TownKind::Merchant => self.merchant_cell,
        }
    }

    pub fn view(&self, kind: TownKind) -> Option<NpcView> {
        let (tex, uv) = match kind {
            TownKind::Guide => self.guide?,
            TownKind::Merchant => self.merchant?,
        };
        Some(NpcView { tex, uv })
    }

    pub fn zombie(&self) -> Option<NpcView> {
        let (tex, uv) = self.zombie?;
        Some(NpcView { tex, uv })
    }

    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_w, self.cell_h)
    }

    pub fn demon_eye(&self) -> Option<NpcView> {
        let (tex, uv) = self.demon_eye?;
        Some(NpcView { tex, uv })
    }

    pub fn demon_eye_cell(&self) -> (u32, u32) {
        self.demon_cell
    }
}

/// 在出生点旁生成向导。
pub fn spawn_guide(world: &World) -> TownNpc {
    TownNpc::guide_at(world, crate::world::SPAWN_TX + 6)
}

/// 把无家可归的 NPC 填进空房；需要时生成商人。返回提示。
pub fn assign_homes(
    world: &World,
    houses: &mut [HouseSlot],
    npcs: &mut Vec<TownNpc>,
) -> Option<String> {
    // 先让已有 NPC 入住空房。
    for house_i in 0..houses.len() {
        if houses[house_i].occupied.is_some() {
            continue;
        }
        if let Some(npc) = npcs.iter_mut().find(|n| n.home.is_none()) {
            let stand = houses[house_i].stand;
            let kind = npc.kind;
            npc.move_into(house_i, stand);
            houses[house_i].occupied = Some(kind);
            return Some(format!("{} 搬进了新房子", kind.label()));
        }
    }

    // 仍有空房且尚无商人 → 生成商人并入住。
    let has_merchant = npcs.iter().any(|n| n.kind == TownKind::Merchant);
    let empty = houses.iter().position(|h| h.occupied.is_none());
    if !has_merchant {
        if let Some(house_i) = empty {
            let stand = houses[house_i].stand;
            let mut m = TownNpc::at_surface(world, TownKind::Merchant, stand.0);
            m.move_into(house_i, stand);
            houses[house_i].occupied = Some(TownKind::Merchant);
            npcs.push(m);
            return Some("商人听说有空房，搬来了".into());
        }
    }
    None
}
