//! 内容声明路径：`terraria::blocks::Dirt`。

use std::fmt;
use std::sync::Arc;

use spark_core::{ErrorArg, ErrorArgs};

/// 内容路径解析错误（稳定码；无用户句子）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentPathError {
    Empty,
    Invalid { path: String },
}

impl ContentPathError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty => "tr.types.content_path.empty",
            Self::Invalid { .. } => "tr.types.content_path.invalid",
        }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Empty => ErrorArgs::new(),
            Self::Invalid { path } => {
                ErrorArgs::new().with("path", ErrorArg::String(Arc::from(path.as_str())))
            }
        }
    }
}

impl fmt::Display for ContentPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ContentPathError {}

/// 稳定内容身份（作者符号，非数字 ID，非 `mod:key` 字符串主身份）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentPath {
    segments: Arc<[Arc<str>]>,
}

impl ContentPath {
    /// 从 `terraria::blocks::Dirt` 或已分段切片构造。
    pub fn parse(path: &str) -> Result<Self, ContentPathError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(ContentPathError::Empty);
        }
        let parts: Vec<Arc<str>> = trimmed
            .split("::")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(Arc::from)
            .collect();
        if parts.is_empty() {
            return Err(ContentPathError::Empty);
        }
        for part in &parts {
            if !is_ident(part) {
                return Err(ContentPathError::Invalid {
                    path: trimmed.to_string(),
                });
            }
        }
        Ok(Self {
            segments: Arc::from(parts),
        })
    }

    pub fn from_segments(
        segments: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> Result<Self, ContentPathError> {
        let parts: Vec<Arc<str>> = segments.into_iter().map(Into::into).collect();
        if parts.is_empty() {
            return Err(ContentPathError::Empty);
        }
        for part in &parts {
            if !is_ident(part) {
                return Err(ContentPathError::Invalid {
                    path: parts
                        .iter()
                        .map(|s| s.as_ref())
                        .collect::<Vec<_>>()
                        .join("::"),
                });
            }
        }
        Ok(Self {
            segments: Arc::from(parts),
        })
    }

    pub fn segments(&self) -> &[Arc<str>] {
        &self.segments
    }

    pub fn name(&self) -> &str {
        self.segments.last().map(|s| s.as_ref()).unwrap_or("")
    }

    pub fn namespace(&self) -> ContentPath {
        if self.segments.len() <= 1 {
            return self.clone();
        }
        Self {
            segments: Arc::from(&self.segments[..self.segments.len() - 1]),
        }
    }
}

impl fmt::Display for ContentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }
            f.write_str(seg)?;
        }
        Ok(())
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
