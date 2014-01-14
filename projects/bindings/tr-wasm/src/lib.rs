#![warn(missing_docs)]
//! Terraria **Wasm** 绑定（`wasm32-unknown-unknown`）。
//!
//! 浏览器 / WASI 侧加载。桌面游玩仍须经 `@game-gpt/terraria` → `tr-napi`。
//!
//! ```text
//! cargo build -p tr-wasm --target wasm32-unknown-unknown --release
//! ```

/// npm 平台包名。
pub const NPM_PLATFORM_PACKAGE: &str = "tr-unknown-wasm32";

#[unsafe(no_mangle)]
pub extern "C" fn tr_version_code() -> u32 {
    parse_version_code(env!("CARGO_PKG_VERSION"))
}

#[unsafe(no_mangle)]
pub extern "C" fn tr_vec2_length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn parse_version_code(v: &str) -> u32 {
    let mut parts = v.split('.');
    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    major.saturating_mul(1_000_000) + minor.saturating_mul(1_000) + patch
}
