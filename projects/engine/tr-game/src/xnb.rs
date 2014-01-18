//! XNB 版本 5 的 `Texture2D` / `SoundEffect` 解码。只读内存，不落盘。
//!
//! 压缩标志 `0x80` 的载荷按帧切块：首字节为 `0xFF` 时随后是大端帧长与块长，否则两字节是块长、帧长固定 32KiB。窗口 64KiB。

use std::path::Path;

use lzxd::{Lzxd, WindowSize};

/// 直通 alpha 的 RGBA8。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaTexture {
    /// 来源文件名，不含目录。
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// 解码后的 PCM（交错 f32，约 -1..=1）。
#[derive(Debug, Clone)]
pub struct PcmSound {
    /// 来源文件名，不含目录。
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

/// 从 XNB 字节解码第一级 `Texture2D`。失败返回可读原因。
pub fn decode_texture_xnb(bytes: &[u8]) -> Result<RgbaTexture, String> {
    decode_texture_xnb_named(bytes, "texture.xnb")
}

pub fn decode_texture_file(path: &Path) -> Result<RgbaTexture, String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("texture.xnb")
        .to_string();
    let bytes = std::fs::read(path).map_err(|e| format!("无法读取 {name}：{e}"))?;
    let mut tex = decode_texture_xnb_named(&bytes, &name)?;
    tex.name = name;
    Ok(tex)
}

/// 从文件解码 `SoundEffect` XNB。
pub fn decode_sound_file(path: &Path) -> Result<PcmSound, String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sound.xnb")
        .to_string();
    let bytes = std::fs::read(path).map_err(|e| format!("无法读取 {name}：{e}"))?;
    let mut snd = decode_sound_xnb_named(&bytes, &name)?;
    snd.name = name;
    Ok(snd)
}

/// 去掉 LZX，写成未压缩 XNB。已经未压缩的文件原样规范化头标志。
pub fn unpack_xnb(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let raw = read_xnb(bytes, "xnb")?;
    let mut out = Vec::with_capacity(10 + raw.body.len());
    out.extend_from_slice(b"XNB");
    out.push(raw.platform);
    out.push(raw.version);
    out.push(0);
    let size = (10 + raw.body.len()) as u32;
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&raw.body);
    Ok(out)
}

struct XnbRaw {
    platform: u8,
    version: u8,
    body: Vec<u8>,
}

fn read_xnb(bytes: &[u8], name: &str) -> Result<XnbRaw, String> {
    if bytes.len() < 10 || &bytes[0..3] != b"XNB" {
        return Err(format!("{name} 不是 XNB"));
    }
    if bytes[3] != b'w' {
        return Err(format!("{name} 不是 Windows XNB"));
    }
    if bytes[4] != 5 {
        return Err(format!("{name} 的 XNB 版本不是 5"));
    }
    let flags = bytes[5];
    let file_size = u32::from_le_bytes(bytes[6..10].try_into().unwrap()) as usize;
    if file_size != bytes.len() {
        return Err(format!(
            "{name} 声明长度 {file_size} 与实际 {} 不符",
            bytes.len()
        ));
    }

    let body = if flags & 0x80 != 0 {
        if flags & 0x40 != 0 {
            return Err(format!("{name} 同时标记了两种压缩"));
        }
        if bytes.len() < 14 {
            return Err(format!("{name} 压缩头被截断"));
        }
        let decompressed_size = u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize;
        decompress_lzx(&bytes[14..], decompressed_size).map_err(|e| format!("{name}：{e}"))?
    } else if flags & 0x40 != 0 {
        return Err(format!("{name} 使用了不支持的压缩"));
    } else {
        bytes[10..].to_vec()
    };
    Ok(XnbRaw {
        platform: bytes[3],
        version: bytes[4],
        body,
    })
}

fn decode_texture_xnb_named(bytes: &[u8], name: &str) -> Result<RgbaTexture, String> {
    let body = read_xnb(bytes, name)?.body;
    let mut cur = Cur { data: &body, pos: 0 };
    let reader_count = cur.read_7bit()?;
    if reader_count == 0 || reader_count > 64 {
        return Err(format!("{name} 的类型读取器数量非法"));
    }
    let mut readers = Vec::with_capacity(reader_count as usize);
    for _ in 0..reader_count {
        readers.push(cur.read_string()?);
        let _version = cur.read_i32()?;
    }
    let shared = cur.read_7bit()?;
    if shared != 0 {
        return Err(format!("{name} 含共享资源，当前只解码独立贴图"));
    }
    let type_id = cur.read_7bit()?;
    if type_id == 0 {
        return Err(format!("{name} 主对象为空"));
    }
    let reader = readers
        .get((type_id - 1) as usize)
        .ok_or_else(|| format!("{name} 的类型编号越界"))?;
    if !reader.contains("Texture2DReader") {
        return Err(format!("{name} 不是 Texture2D"));
    }

    let format = cur.read_i32()?;
    let width = cur.read_u32()?;
    let height = cur.read_u32()?;
    let mip_count = cur.read_u32()?;
    if mip_count == 0 {
        return Err(format!("{name} 没有 mip"));
    }
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err(format!("{name} 尺寸非法：{width}x{height}"));
    }
    let data_size = cur.read_u32()? as usize;
    let pixels = cur.read_bytes(data_size)?;
    if format != 0 {
        return Err(format!(
            "{name} 的 SurfaceFormat {format} 不是 Color。当前只解码未压缩的 32 位贴图"
        ));
    }
    let expect = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| format!("{name} 尺寸溢出"))?;
    if pixels.len() != expect {
        return Err(format!(
            "{name} 像素字节 {} 与 {width}x{height} 不符",
            pixels.len()
        ));
    }
    let mut rgba = pixels.to_vec();
    unpremultiply(&mut rgba);
    Ok(RgbaTexture {
        name: name.to_string(),
        width,
        height,
        rgba,
    })
}

fn decode_sound_xnb_named(bytes: &[u8], name: &str) -> Result<PcmSound, String> {
    let body = read_xnb(bytes, name)?.body;
    let mut cur = Cur { data: &body, pos: 0 };
    let reader_count = cur.read_7bit()?;
    if reader_count == 0 || reader_count > 64 {
        return Err(format!("{name} 的类型读取器数量非法"));
    }
    let mut readers = Vec::with_capacity(reader_count as usize);
    for _ in 0..reader_count {
        readers.push(cur.read_string()?);
        let _version = cur.read_i32()?;
    }
    let shared = cur.read_7bit()?;
    if shared != 0 {
        return Err(format!("{name} 含共享资源，当前只解码独立音效"));
    }
    let type_id = cur.read_7bit()?;
    if type_id == 0 {
        return Err(format!("{name} 主对象为空"));
    }
    let reader = readers
        .get((type_id - 1) as usize)
        .ok_or_else(|| format!("{name} 的类型编号越界"))?;
    if !reader.contains("SoundEffectReader") {
        return Err(format!("{name} 不是 SoundEffect"));
    }

    let fmt_len = cur.read_u32()? as usize;
    if fmt_len < 16 || fmt_len > 128 {
        return Err(format!("{name} 的 WAVEFORMATEX 长度非法：{fmt_len}"));
    }
    let fmt = cur.read_bytes(fmt_len)?;
    let format_tag = u16::from_le_bytes(fmt[0..2].try_into().unwrap());
    let channels = u16::from_le_bytes(fmt[2..4].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
    let bits = u16::from_le_bytes(fmt[14..16].try_into().unwrap());
    if format_tag != 1 {
        return Err(format!("{name} 不是 PCM（format={format_tag}）"));
    }
    if channels == 0 || channels > 2 {
        return Err(format!("{name} 声道数非法：{channels}"));
    }
    if sample_rate < 8000 || sample_rate > 192_000 {
        return Err(format!("{name} 采样率非法：{sample_rate}"));
    }
    if bits != 8 && bits != 16 {
        return Err(format!("{name} 位深非法：{bits}"));
    }

    let data_len = cur.read_u32()? as usize;
    if data_len == 0 || data_len > 16 * 1024 * 1024 {
        return Err(format!("{name} 波形长度非法：{data_len}"));
    }
    let raw = cur.read_bytes(data_len)?;
    let samples = pcm_bytes_to_f32(raw, bits)?;
    Ok(PcmSound {
        name: name.to_string(),
        sample_rate,
        channels,
        samples,
    })
}

fn pcm_bytes_to_f32(raw: &[u8], bits: u16) -> Result<Vec<f32>, String> {
    match bits {
        8 => Ok(raw
            .iter()
            .map(|&b| (b as f32 - 128.0) / 128.0)
            .collect()),
        16 => {
            if raw.len() % 2 != 0 {
                return Err("16 位 PCM 字节数为奇数".into());
            }
            Ok(raw
                .chunks_exact(2)
                .map(|c| {
                    let v = i16::from_le_bytes([c[0], c[1]]);
                    v as f32 / 32768.0
                })
                .collect())
        }
        _ => Err(format!("不支持的位深 {bits}")),
    }
}

fn decompress_lzx(src: &[u8], decompressed_size: usize) -> Result<Vec<u8>, String> {
    if decompressed_size > 64 * 1024 * 1024 {
        return Err("解压后超过 64MiB".into());
    }
    let mut lzxd = Lzxd::new(WindowSize::KB64);
    let mut out = Vec::with_capacity(decompressed_size);
    let mut pos = 0usize;
    while pos < src.len() && out.len() < decompressed_size {
        let (frame_size, block_size, header) = if src[pos] == 0xFF {
            if pos + 5 > src.len() {
                return Err("LZX 帧头被截断".into());
            }
            let frame_size = u16::from_be_bytes([src[pos + 1], src[pos + 2]]) as usize;
            let block_size = u16::from_be_bytes([src[pos + 3], src[pos + 4]]) as usize;
            (frame_size, block_size, 5usize)
        } else {
            if pos + 2 > src.len() {
                return Err("LZX 块头被截断".into());
            }
            let block_size = u16::from_be_bytes([src[pos], src[pos + 1]]) as usize;
            (0x8000usize, block_size, 2usize)
        };
        if block_size == 0 || frame_size == 0 {
            break;
        }
        if block_size > 0x10000 || frame_size > 0x10000 {
            return Err("LZX 块尺寸非法".into());
        }
        let start = pos + header;
        let end = start
            .checked_add(block_size)
            .ok_or_else(|| "LZX 块越界".to_string())?;
        if end > src.len() {
            return Err("LZX 块超出文件".into());
        }
        let piece = lzxd
            .decompress_next(&src[start..end], frame_size)
            .map_err(|e| format!("LZX 解压失败：{e}"))?;
        let need = decompressed_size - out.len();
        let n = piece.len().min(need);
        out.extend_from_slice(&piece[..n]);
        pos = end;
    }
    if out.len() != decompressed_size {
        return Err(format!(
            "解压长度 {} 与声明的 {decompressed_size} 不符",
            out.len()
        ));
    }
    Ok(out)
}

/// XNA `Color` 存的是预乘 alpha。渲染器使用直通 alpha。
fn unpremultiply(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3];
        if a == 0 || a == 255 {
            continue;
        }
        let inv = 255.0 / f32::from(a);
        for c in 0..3 {
            let v = (f32::from(px[c]) * inv).ceil();
            px[c] = v.min(255.0) as u8;
        }
    }
}

struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn read_u8(&mut self) -> Result<u8, String> {
        let b = *self.data.get(self.pos).ok_or("XNB 被截断")?;
        self.pos += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| "XNB 读取越界".to_string())?;
        let slice = self.data.get(self.pos..end).ok_or("XNB 被截断")?;
        self.pos = end;
        Ok(slice)
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        Ok(self.read_u32()? as i32)
    }

    fn read_7bit(&mut self) -> Result<u32, String> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            if shift > 28 {
                return Err("XNB 7-bit 整数过长".into());
            }
            let value = self.read_u8()?;
            result |= u32::from(value & 0x7F) << shift;
            if value & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_7bit()? as usize;
        if len > 1024 {
            return Err("XNB 字符串过长".into());
        }
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| "XNB 字符串不是 UTF-8".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn decodes_uncompressed_color_texture() {
        let reader = b"Microsoft.Xna.Framework.Content.Texture2DReader";
        let mut body = Vec::new();
        push_7bit(&mut body, 1);
        push_7bit(&mut body, reader.len() as u32);
        body.extend_from_slice(reader);
        body.extend_from_slice(&0i32.to_le_bytes());
        push_7bit(&mut body, 0);
        push_7bit(&mut body, 1);
        body.extend_from_slice(&0i32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&4u32.to_le_bytes());
        // 预乘：红 128、alpha 128 → 直通红 255。
        body.extend_from_slice(&[128, 0, 0, 128]);

        let mut file = Vec::new();
        file.extend_from_slice(b"XNBw");
        file.push(5);
        file.push(0x01);
        let size = (10 + body.len()) as u32;
        file.extend_from_slice(&size.to_le_bytes());
        file.extend_from_slice(&body);

        let tex = decode_texture_xnb(&file).unwrap();
        assert_eq!((tex.width, tex.height), (1, 1));
        assert_eq!(tex.rgba, vec![255, 0, 0, 128]);
        let unpacked = unpack_xnb(&file).unwrap();
        let again = decode_texture_xnb(&unpacked).unwrap();
        assert_eq!(again.rgba, tex.rgba);
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(decode_texture_xnb(b"XNB").is_err());
    }

    #[test]
    fn decodes_install_tiles0_when_env_set() {
        let Ok(root) = std::env::var("TR_ORIGINAL") else {
            return;
        };
        let path = std::path::Path::new(&root)
            .join("Content")
            .join("Images")
            .join("Tiles_0.xnb");
        let tex = decode_texture_file(&path).expect("Tiles_0");
        assert!(tex.width > 0 && tex.height > 0);
        assert_eq!(tex.rgba.len(), (tex.width * tex.height * 4) as usize);
        assert!(tex.rgba.chunks_exact(4).any(|px| px[3] > 0));
    }
}
