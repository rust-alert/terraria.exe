#![warn(missing_docs)]
//! Terraria **Node-API** 绑定。
//!
//! 产品唯一启动链：
//!
//! ```text
//! terraria emulate --path <原版安装目录>
//!   → @game-gpt/terraria (CLI)
//!   → tr-napi (本 crate, feature = node)
//!   → tr-game::run_emulate
//! ```
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
