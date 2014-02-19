//! napi 导出（feature = `node`）。

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::host::TerrariaJsHost;

#[napi(object)]
pub struct JsHostInfo {
    pub name: String,
    pub version: String,
    pub npm_package: String,
}

#[napi]
pub struct JsTerrariaHost {
    inner: TerrariaJsHost,
}

#[napi]
impl JsTerrariaHost {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: TerrariaJsHost::new(),
        }
    }

    #[napi]
    pub fn info(&self) -> JsHostInfo {
        let i = self.inner.info();
        JsHostInfo {
            name: i.name.into(),
            version: i.version.into(),
            npm_package: i.npm_package.into(),
        }
    }

    #[napi]
    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        self.inner.vec2_length(x, y)
    }

    #[napi]
    pub fn block_count(&self) -> u32 {
        self.inner.block_count()
    }

    /// 校验 `--path` 是否为 Terraria 安装根（含 `Content/`）。
    #[napi]
    pub fn validate_path(&self, path: String) -> Result<()> {
        self.inner.validate_path(&path).map_err(Error::from_reason)
    }

    /// `terraria emulate --path`：阻塞运行至窗口关闭。
    #[napi]
    pub fn emulate(&self, path: String) -> Result<()> {
        self.inner.emulate(&path).map_err(Error::from_reason)
    }

    /// `terraria unpack`：写出未压缩 XNB。
    #[napi]
    pub fn unpack(&self, path: String, out: String, only: Vec<String>) -> Result<String> {
        self.inner
            .unpack(&path, &out, &only)
            .map_err(Error::from_reason)
    }

    /// `terraria extract`：把贴图写成 PNG。
    #[napi]
    pub fn extract(&self, path: String, out: String, only: Vec<String>) -> Result<String> {
        self.inner
            .extract(&path, &out, &only)
            .map_err(Error::from_reason)
    }
}
