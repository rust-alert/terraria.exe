#![warn(missing_docs)]
//! Terraria 游戏逻辑库。
//!
//! **禁止** `cargo run` / 独立 `main` 启动。产品入口是 npm CLI：
//!
//! ```text
//! terraria emulate --path <原版 Terraria 安装目录>
//! terraria unpack --path <安装根> --out <目录>
//! terraria extract --path <安装根> --out <目录>
//! ```
//!
//! 窗口只由 `emulate` 打开。`unpack` / `extract` 不启动游戏。
//! 由 `@game-gpt/terraria` → `tr-napi` → 本库。

mod aim;
mod app;
mod container;
mod content_boot;
mod content_export;
mod content_recipes;
mod content_tiles;
mod craft;
mod demo;
mod enemy;
mod fx;
mod grapple;
mod hud;
mod hud_chrome;
mod housing;
mod icons;
mod install;
mod lightmap;
mod npc;
mod palette;
mod play;
mod player;
mod postprocess;
mod proof;
mod save;
mod screens;
mod sfx;
mod sheets;
mod shop;
mod sky;
mod terrain_edges;
mod tile_frame;
mod tiles;
mod trees;
mod use_item;
mod weapon;
mod world;
mod world_view;
mod worldgen;
mod xnb;

use std::path::{Path, PathBuf};

use spark_engine::run_game;
use spark_renderer::WindowConfig;

use crate::app::TerrariaApp;
pub use crate::content_boot::{MOD_LOAD_STATUS, ModLoad, load_original_mods};
pub use crate::content_export::{extract_content, unpack_content};

/// `terraria emulate --path` 的运行参数。
#[derive(Debug, Clone)]
pub struct EmulateOptions {
    /// 正版 Terraria 安装根目录（须含 `Content/`）。
    pub original_path: PathBuf,
    /// 直接进入游玩（跳过标题）。默认 true。
    pub boot_play: bool,
}

impl EmulateOptions {
    pub fn new(original_path: impl Into<PathBuf>) -> Self {
        Self {
            original_path: original_path.into(),
            boot_play: true,
        }
    }
}

/// 校验正版安装目录。失败时返回可读错误（供 CLI / napi）。
pub fn validate_original_install(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "原版路径不是目录：{}。请传入已安装的 Terraria 根目录。",
            path.display()
        ));
    }
    let content = path.join("Content");
    if !content.is_dir() {
        return Err(format!(
            "未找到 Content/：{}。必须指向正版 Terraria 安装根（含 Content）。",
            path.display()
        ));
    }
    let images = content.join("Images");
    if !images.is_dir() {
        return Err(format!(
            "未找到 Content/Images/：{}。请确认这是完整正版安装。",
            path.display()
        ));
    }
    Ok(())
}

/// 唯一游戏启动入口：校验原版路径后打开窗口并阻塞至退出。
pub fn run_emulate(opts: EmulateOptions) -> Result<(), String> {
    validate_original_install(&opts.original_path)?;

    // 供内容 / 资产管线读取正版根。禁止把盘符写进仓库。
    // SAFETY: 单线程启动前设置，供本进程后续读取。
    unsafe {
        std::env::set_var("TR_CONTENT", &opts.original_path);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        path = %opts.original_path.display(),
        "terraria emulate：正版路径已确认"
    );

    let assets = crate::content_boot::boot_content(&opts.original_path)?;
    let mut host = TerrariaApp::new();
    host.content_assets = assets;
    if opts.boot_play {
        host.boot_into_play();
    }

    run_game(
        WindowConfig {
            title: "Terraria (Rust rewrite)".into(),
            width: 1280,
            height: 720,
            clear_color: [0.04, 0.07, 0.14, 1.0],
        },
        host,
    )
    .map_err(|e| e.to_string())
}
