//! JS / 测试共用的宿主门面。

use std::path::PathBuf;

use spark_core::Vec2;

use tr_game::{
    EmulateOptions, extract_content, run_emulate, unpack_content, validate_original_install,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub npm_package: &'static str,
}

impl Default for HostInfo {
    fn default() -> Self {
        Self {
            name: "Terraria",
            version: env!("CARGO_PKG_VERSION"),
            npm_package: crate::NPM_PACKAGE_NAME,
        }
    }
}

/// JS 宿主。游戏窗口只能经 [`TerrariaJsHost::emulate`] 拉起。
#[derive(Debug, Default)]
pub struct TerrariaJsHost;

impl TerrariaJsHost {
    pub fn new() -> Self {
        Self
    }

    pub fn info(&self) -> HostInfo {
        HostInfo::default()
    }

    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        let v = Vec2::new(x as f32, y as f32);
        (v.x * v.x + v.y * v.y).sqrt() as f64
    }

    pub fn block_count(&self) -> u32 {
        tr_core::try_content()
            .map(|c| c.block_count() as u32)
            .unwrap_or(0)
    }

    /// 校验安装根（不启动窗口）。
    pub fn validate_path(&self, path: &str) -> Result<(), String> {
        validate_original_install(PathBuf::from(path).as_path())
    }

    /// 唯一启动：阻塞直至窗口关闭。
    pub fn emulate(&self, path: &str) -> Result<(), String> {
        run_emulate(EmulateOptions::new(path))
    }

    /// 把 Content XNB 解成未压缩 XNB。`out` 必须在仓库和安装目录之外。
    pub fn unpack(&self, path: &str, out: &str, only: &[String]) -> Result<String, String> {
        validate_original_install(PathBuf::from(path).as_path())?;
        unpack_content(
            PathBuf::from(path).as_path(),
            PathBuf::from(out).as_path(),
            only,
        )
    }

    /// 把 `Texture2D` 写成 PNG。PNG 编码走 `spark-image`。
    pub fn extract(&self, path: &str, out: &str, only: &[String]) -> Result<String, String> {
        validate_original_install(PathBuf::from(path).as_path())?;
        extract_content(
            PathBuf::from(path).as_path(),
            PathBuf::from(out).as_path(),
            only,
        )
    }
}
