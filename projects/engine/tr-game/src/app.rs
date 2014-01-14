//! 应用壳：状态与主循环入口。玩法/绘制拆到同目录其它模块。

use spark_audio::AudioBus;
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};
use tr_core::{ItemId, RaceId};

use crate::content_boot::ContentAssets;
use crate::enemy::{Enemy, spawn_surface_slimes};
use crate::fx::{DamageFloater, DustParticle};
use crate::icons::IconAtlas;
use crate::objective::Objective;
use crate::player::{ItemStack, Player, PlayerAtlas};
use crate::portal::PortalState;
use crate::sky::SkyAtlas;
use crate::tiles::TileAtlas;
use crate::weapon::Projectile;
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Title,
    NewGame,
    Playing,
    Pause,
}

pub struct TerrariaApp {
    pub(crate) screen: Screen,
    pub(crate) exit: bool,
    pub(crate) race: RaceId,
    pub(crate) world: Option<World>,
    pub(crate) player: Option<Player>,
    pub(crate) cam_x: f32,
    pub(crate) cam_y: f32,
    pub(crate) toast: String,
    pub(crate) toast_t: f32,
    pub(crate) screen_w: f32,
    pub(crate) screen_h: f32,
    pub(crate) mouse: (f32, f32),
    pub(crate) demo_t: f32,
    pub(crate) demo: bool,
    pub(crate) demo_phase: u8,
    pub(crate) status_acc: f32,
    /// 手搓 / 工作台面板。
    pub(crate) craft_open: bool,
    pub(crate) enemies: Vec<Enemy>,
    /// 世界日时（秒），周期约 180s。
    pub(crate) day_t: f32,
    /// 是否已发现过裂痕锚（叙事提示一次）。
    pub(crate) seen_warp: bool,
    /// 是否成功传送过至少一次。
    pub(crate) warped_once: bool,
    /// 是否已睡过一夜或熬过破晓（住所循环）。
    pub(crate) survived_night: bool,
    /// 本周期是否已提示「入夜」。
    pub(crate) night_warned: bool,
    /// 黑暗压力累计（秒），靠近光源时衰减。
    pub(crate) dark_stress: f32,
    pub(crate) objective: Objective,
    /// I 键背包面板。
    pub(crate) bag_open: bool,
    /// M 键世界地图面板。
    pub(crate) map_open: bool,
    /// 程序化提示音（设备不可用时静默）。
    pub(crate) audio: AudioBus,
    /// 裂痕传送门会话状态。
    pub(crate) portal: PortalState,
    /// 打开的木箱格坐标。
    pub(crate) chest_open: Option<(i32, i32)>,
    /// 玩家投射物。
    pub(crate) projectiles: Vec<Projectile>,
    /// F3：调试信息叠层。
    pub(crate) debug_hud: bool,
    /// HUD 热键图标图集（程序化占位）。
    pub(crate) icon_atlas: IconAtlas,
    /// 世界瓦片图集。
    pub(crate) tile_atlas: TileAtlas,
    /// 天空视差贴图层（PNG 优先）。
    pub(crate) sky_atlas: SkyAtlas,
    /// 玩家像素条带。
    pub(crate) player_atlas: PlayerAtlas,
    /// 伤害飘字。
    pub(crate) damage_fx: Vec<DamageFloater>,
    /// 落地 / 挖掘尘粒。
    pub(crate) dust_fx: Vec<DustParticle>,
    /// 内容包解析出的 PNG 路径。
    pub(crate) content_assets: ContentAssets,
    /// 鼠标拖着的物品堆（背包/箱子光标）。
    pub(crate) cursor_stack: Option<ItemStack>,
}

impl TerrariaApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Title,
            exit: false,
            race: RaceId::HUMAN,
            world: None,
            player: None,
            cam_x: 0.0,
            cam_y: 0.0,
            toast: String::new(),
            toast_t: 0.0,
            screen_w: 1280.0,
            screen_h: 720.0,
            mouse: (0.0, 0.0),
            demo_t: 0.0,
            demo: false,
            demo_phase: 0,
            status_acc: 0.0,
            craft_open: false,
            enemies: Vec::new(),
            day_t: 40.0,
            seen_warp: false,
            warped_once: false,
            survived_night: false,
            night_warned: false,
            dark_stress: 0.0,
            objective: Objective::GatherWood,
            bag_open: false,
            map_open: false,
            audio: AudioBus::try_open(),
            portal: PortalState::default(),
            chest_open: None,
            projectiles: Vec::new(),
            debug_hud: false,
            icon_atlas: IconAtlas::new(),
            tile_atlas: TileAtlas::new(),
            sky_atlas: SkyAtlas::new(),
            player_atlas: PlayerAtlas::new(),
            damage_fx: Vec::new(),
            dust_fx: Vec::new(),
            content_assets: ContentAssets::empty(),
            cursor_stack: None,
        }
    }

    /// 跳过标题，直接进入地表（调试 / `--play`）。
    pub fn boot_into_play(&mut self) {
        self.start_game();
    }

    /// 无驱动竖切：移动 / 挖掘 / 放置后写状态文件并退出。
    pub fn boot_demo(&mut self) {
        self.demo = true;
        self.start_game();
    }

    pub(crate) fn start_game(&mut self) {
        let world = World::generate(0xA57A_C2AF);
        let (sx, sy) = world.spawn_pos();
        let mut enemies = Vec::new();
        spawn_surface_slimes(&world, &mut enemies);
        self.player = Some(Player::new(sx, sy));
        if let Some(p) = self.player.as_mut() {
            let _ = p.inv.add(ItemId::CLOTH_BAG, 1);
            let _ = p.inv.add(ItemId::WOOD, 8);
            let _ = p.inv.add(ItemId::DIRT, 12);
            let _ = p.inv.add(ItemId::TORCH, 4);
            p.select_item(ItemId::WOOD);
        }
        self.cam_x = sx - 640.0;
        self.cam_y = sy - 360.0;
        self.world = Some(world);
        self.enemies = enemies;
        self.craft_open = false;
        self.bag_open = false;
        self.map_open = false;
        self.seen_warp = false;
        self.warped_once = false;
        self.survived_night = false;
        self.night_warned = false;
        self.dark_stress = 0.0;
        self.objective = Objective::GatherWood;
        self.portal = PortalState::default();
        self.chest_open = None;
        self.cursor_stack = None;
        self.projectiles.clear();
        self.damage_fx.clear();
        self.dust_fx.clear();
        self.screen = Screen::Playing;
        self.set_toast("苏醒。先砍树取木，靠近逃生舱按 E 休整，再造工作台与木镐。");
        tracing::info!(x = sx, y = sy, n_enemy = self.enemies.len(), "进入地表");
    }

    pub(crate) fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = msg.into();
        self.toast_t = 4.5;
    }
}

impl GameHost for TerrariaApp {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        self.screen_w = frame.screen_w;
        self.screen_h = frame.screen_h;
        self.mouse = frame.input.mouse_pos();
        match self.screen {
            Screen::Title => self.ui_title(frame.input),
            Screen::NewGame => self.ui_new_game(frame.input),
            Screen::Playing => self.update_playing(frame),
            Screen::Pause => {
                if frame.input.key_pressed(Key::Escape) {
                    self.screen = Screen::Playing;
                }
                self.ui_pause(frame.input);
            }
        }
    }

    fn draw(&mut self, draw: &mut DrawList) {
        match self.screen {
            Screen::Title => self.paint_title(draw),
            Screen::NewGame => self.paint_new_game(draw),
            Screen::Playing => {
                self.icon_atlas.ensure(draw, &self.content_assets);
                self.tile_atlas.ensure(draw, &self.content_assets);
                self.sky_atlas.ensure(draw, &self.content_assets);
                self.player_atlas.ensure(draw);
                draw.begin_world();
                self.draw_world(draw);
                draw.begin_hud();
                self.paint_hud(draw);
            }
            Screen::Pause => {
                self.icon_atlas.ensure(draw, &self.content_assets);
                self.tile_atlas.ensure(draw, &self.content_assets);
                self.sky_atlas.ensure(draw, &self.content_assets);
                self.player_atlas.ensure(draw);
                draw.begin_world();
                self.draw_world(draw);
                draw.begin_hud();
                self.paint_pause(draw);
            }
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
