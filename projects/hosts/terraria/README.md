# `@game-gpt/terraria`

Terraria Rust rewrite 的 npm 元包。

## 唯一启动方式

```bash
terraria emulate --path "D:/SteamLibrary/steamapps/common/Terraria"
```

`--path` 必须是**正版** Terraria 安装根目录（含 `Content/`）。没有原版无法运行。

不要使用 `cargo run`。游戏窗口由本包经 `tr-napi` 拉起。
