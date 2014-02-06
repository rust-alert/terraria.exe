//! 安装根路径与用户可见文案的唯一出口。
//!
//! 其它模块禁止再写暗示素材来源的散落措辞。
//! 对外只说「Terraria 安装根」（须含 `Content/`）。

use std::path::Path;

/// CLI / 文档里的路径占位。
pub const PATH_PLACEHOLDER: &str = "<Terraria 安装根>";

/// `--path` 字段说明（短）。
pub const PATH_FIELD_DOC: &str = "Terraria 安装根目录（须含 Content/）";

/// 校验安装根。失败返回可读错误。
pub fn validate(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "路径不是目录：{}。请传入 Terraria 安装根（含 Content/）。",
            path.display()
        ));
    }
    let content = path.join("Content");
    if !content.is_dir() {
        return Err(format!(
            "未找到 Content/：{}。须指向 Terraria 安装根。",
            path.display()
        ));
    }
    let images = content.join("Images");
    if !images.is_dir() {
        return Err(format!(
            "未找到 Content/Images/：{}。请确认安装完整。",
            path.display()
        ));
    }
    Ok(())
}

/// `emulate` 启动日志。
pub fn log_path_confirmed(path: &Path) -> String {
    format!("terraria emulate：安装根已确认 {}", path.display())
}

/// 缺 `Content/`（导出管线）。
pub fn err_missing_content(path: &Path) -> String {
    format!(
        "未找到 Content/：{}。须指向 Terraria 安装根。",
        path.display()
    )
}

/// 缺贴图目录。
pub fn err_missing_images(path: &Path) -> String {
    format!(
        "未找到贴图目录：{}。须为安装根下的 Content/Images。",
        path.display()
    )
}

/// 输出目录落在安装根内。
pub fn err_out_inside_install() -> String {
    "输出目录不能放在 Terraria 安装目录里。".into()
}

/// 证明贴图缺失。
pub fn err_missing_proof(kind: &str) -> String {
    format!("缺少 {kind}。无法确认安装 Content 可读。")
}
