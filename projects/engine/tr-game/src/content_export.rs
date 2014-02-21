//! `unpack` / `extract`。
//!
//! `unpack` 写出未压缩 XNB。`extract` 把 `Texture2D` 交给 `PixelImage::save_png`。
//! 输出不能落在本仓库或 Terraria 安装目录里。

use std::path::{Path, PathBuf};

use spark_image::PixelImage;

use crate::xnb::{decode_texture_file, unpack_xnb};

/// 一次导出的计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReport {
    pub written: u32,
    pub skipped: u32,
}

impl ExportReport {
    fn line(self, verb: &str) -> String {
        format!("{verb}：写出 {}，跳过 {}", self.written, self.skipped)
    }
}

/// 把安装里的 XNB 解成未压缩 XNB。`only` 非空时，相对路径须包含其中任一字串。
pub fn unpack_content(install: &Path, out: &Path, only: &[String]) -> Result<String, String> {
    let files = list_xnb(install, only)?;
    guard_output(install, out)?;
    let mut report = ExportReport {
        written: 0,
        skipped: 0,
    };
    for src in files {
        let rel = src
            .strip_prefix(install)
            .map_err(|_| format!("路径不在安装根内：{}", src.display()))?;
        let dst = out.join(rel);
        let bytes = std::fs::read(&src).map_err(|e| format!("无法读取 {}：{e}", src.display()))?;
        let plain = unpack_xnb(&bytes).map_err(|e| format!("{}：{e}", src.display()))?;
        write_bytes(&dst, &plain)?;
        report.written += 1;
    }
    if report.written == 0 {
        return Err("没有写出任何 XNB。请检查 --only。".into());
    }
    Ok(report.line("unpack"))
}

/// 把 `Texture2D` 写成 PNG。不是贴图的 XNB 计入跳过。
pub fn extract_content(install: &Path, out: &Path, only: &[String]) -> Result<String, String> {
    let files = list_xnb(install, only)?;
    guard_output(install, out)?;
    let mut report = ExportReport {
        written: 0,
        skipped: 0,
    };
    for src in files {
        let rel = src
            .strip_prefix(install)
            .map_err(|_| format!("路径不在安装根内：{}", src.display()))?;
        match decode_texture_file(&src) {
            Ok(tex) => {
                let mut png_rel = rel.to_path_buf();
                png_rel.set_extension("png");
                let dst = out.join(png_rel);
                let image = PixelImage::from_rgba8(tex.width, tex.height, tex.rgba)
                    .map_err(|e| format!("{} 无法交给像素图：{e}", src.display()))?;
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("无法创建目录 {}：{e}", parent.display()))?;
                }
                image
                    .save_png(&dst)
                    .map_err(|e| format!("无法写出 {}：{e}", dst.display()))?;
                report.written += 1;
            }
            Err(err) if err.contains("不是 Texture2D") => {
                report.skipped += 1;
            }
            Err(err) => return Err(format!("{}：{err}", src.display())),
        }
    }
    if report.written == 0 {
        return Err("没有写出任何 PNG。这些 XNB 不是 Texture2D，或 --only 没有命中贴图。".into());
    }
    Ok(report.line("extract"))
}

fn list_xnb(install: &Path, only: &[String]) -> Result<Vec<PathBuf>, String> {
    let content = install.join("Content");
    if !content.is_dir() {
        return Err(crate::install::err_missing_content(install));
    }
    let mut files = Vec::new();
    collect_xnb(&content, &mut files)?;
    if !only.is_empty() {
        files.retain(|path| {
            let text = path.to_string_lossy();
            only.iter()
                .any(|needle| !needle.is_empty() && text.contains(needle))
        });
    }
    files.sort();
    if files.is_empty() {
        return Err("没有匹配的 XNB。".into());
    }
    Ok(files)
}

fn collect_xnb(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("无法列出 {}：{e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| format!("读取目录项失败：{e}"))?;
        let path = ent.path();
        if path.is_dir() {
            collect_xnb(&path, out)?;
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xnb"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建目录 {}：{e}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("无法写出 {}：{e}", path.display()))
}

fn guard_output(install: &Path, out: &Path) -> Result<(), String> {
    if out.as_os_str().is_empty() {
        return Err("缺少 --out。解出的文件不能写进仓库。".into());
    }
    let install = canonical_dir(install)?;
    let out_abs = absolute_new(out);
    if out_abs.starts_with(&install) {
        return Err(crate::install::err_out_inside_install());
    }
    if inside_repo(&out_abs) {
        return Err("输出目录不能放在本仓库里。请改到仓库外。".into());
    }
    Ok(())
}

fn canonical_dir(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map(strip_verbatim)
        .map_err(|e| format!("无法解析目录 {}：{e}", path.display()))
}

fn absolute_new(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    if let Ok(canon) = abs.canonicalize() {
        return strip_verbatim(canon);
    }
    if let Some(parent) = abs.parent() {
        if let Ok(canon) = parent.canonicalize() {
            if let Some(name) = abs.file_name() {
                return strip_verbatim(canon).join(name);
            }
        }
    }
    strip_verbatim(abs)
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

fn inside_repo(path: &Path) -> bool {
    let mut cur = Some(path);
    while let Some(dir) = cur {
        let game = dir
            .join("projects")
            .join("engine")
            .join("tr-game")
            .join("Cargo.toml");
        let host = dir
            .join("projects")
            .join("hosts")
            .join("terraria")
            .join("package.json");
        if game.is_file() && host.is_file() {
            return true;
        }
        cur = dir.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_image::PixelImage;

    fn sample_xnb() -> Vec<u8> {
        let reader = b"Microsoft.Xna.Framework.Content.Texture2DReader";
        let mut body = Vec::new();
        push_7bit(&mut body, 1);
        push_7bit(&mut body, reader.len() as u32);
        body.extend_from_slice(reader);
        body.extend_from_slice(&0i32.to_le_bytes());
        push_7bit(&mut body, 0);
        push_7bit(&mut body, 1);
        body.extend_from_slice(&0i32.to_le_bytes());
        body.extend_from_slice(&2u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&8u32.to_le_bytes());
        body.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]);

        let mut file = Vec::new();
        file.extend_from_slice(b"XNBw");
        file.push(5);
        file.push(0);
        let size = (10 + body.len()) as u32;
        file.extend_from_slice(&size.to_le_bytes());
        file.extend_from_slice(&body);
        file
    }

    fn push_7bit(buf: &mut Vec<u8>, mut n: u32) {
        loop {
            let mut b = (n & 0x7F) as u8;
            n >>= 7;
            if n != 0 {
                b |= 0x80;
            }
            buf.push(b);
            if n == 0 {
                break;
            }
        }
    }

    #[test]
    fn extract_uses_pixel_image_png() {
        let root = std::env::temp_dir().join(format!("tr-export-{}", std::process::id()));
        let install = root.join("install");
        let out = root.join("out");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(install.join("Content").join("Images")).unwrap();
        std::fs::write(
            install.join("Content").join("Images").join("Item_1.xnb"),
            sample_xnb(),
        )
        .unwrap();

        let line = extract_content(&install, &out, &[]).unwrap();
        assert!(line.contains("写出 1"));
        let png = out.join("Content").join("Images").join("Item_1.png");
        let image = PixelImage::load(&png).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.pixel(1, 0).unwrap(), [0, 255, 0, 255]);

        let unpacked = unpack_content(&install, &out, &["Item_1.xnb".into()]).unwrap();
        assert!(unpacked.contains("写出 1"));
        let plain = std::fs::read(out.join("Content").join("Images").join("Item_1.xnb")).unwrap();
        assert!(plain.starts_with(b"XNB"));

        let err = extract_content(&install, &install.join("nested"), &[]).unwrap_err();
        assert!(err.contains("Terraria 安装"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
