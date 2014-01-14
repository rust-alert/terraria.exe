//! `LocaleText`：内容字段上的本地化符号引用。

use std::fmt;
use std::sync::Arc;

use spark_localization::{MessageId, MessageRef, NamespaceId};

/// Locale 消息符号路径（如 `standard.block.dirt`）。
///
/// 内容字段上的本地化符号路径，例如 `standard.block.dirt`。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocaleSymbol {
    path: Arc<str>,
}

impl LocaleSymbol {
    pub fn new(path: impl Into<Arc<str>>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    /// 拆成命名空间 + 消息名（最后一个 `.` 分段）。
    ///
    /// `standard.block.dirt` → namespace `standard`，message `block.dirt`。
    pub fn split_namespace(&self) -> (NamespaceId, MessageId) {
        if let Some((ns, rest)) = self.path.split_once('.') {
            (NamespaceId::new(ns), MessageId::name(rest))
        } else {
            (
                NamespaceId::new("standard"),
                MessageId::name(self.path.as_ref()),
            )
        }
    }

    pub fn to_message_ref(&self) -> MessageRef {
        let (ns, msg) = self.split_namespace();
        MessageRef {
            namespace: ns,
            message: msg,
        }
    }
}

impl fmt::Display for LocaleSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.path)
    }
}

/// 内容 schema 中的本地化文本字段类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocaleText {
    pub symbol: LocaleSymbol,
}

impl LocaleText {
    pub fn new(symbol: impl Into<Arc<str>>) -> Self {
        Self {
            symbol: LocaleSymbol::new(symbol),
        }
    }

    pub fn message_ref(&self) -> MessageRef {
        self.symbol.to_message_ref()
    }
}
