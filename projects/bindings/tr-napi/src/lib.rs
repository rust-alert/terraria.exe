#![warn(missing_docs)]
//! Terraria **Node-API** 绑定。
//!
//! 产品入口：
//!
//! ```text
//! terraria emulate --path <Terraria 安装目录>
//! terraria unpack --path <安装根> --out <目录>
//! terraria extract --path <安装根> --out <目录>
//! ```
//!
//! 窗口只由 `emulate` 打开。
//!
//! ```text
//! cargo build -p tr-napi --release --features node
//! ```

mod host;

#[cfg(feature = "node")]
mod node;

pub use host::{HostInfo, TerrariaJsHost};

/// npm 元包名。
pub const NPM_PACKAGE_NAME: &str = "@game-gpt/terraria";
