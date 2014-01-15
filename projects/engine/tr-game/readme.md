# `tr-game`

Terraria 游戏逻辑库。仅由 `tr-napi` 拉起，不提供独立二进制入口。

不要使用 `cargo run`。游戏窗口由 npm CLI 的 `emulate` 拉起。`unpack` / `extract` 不打开窗口。

```bash
terraria emulate --path <正版 Terraria 安装目录>
terraria unpack --path <正版 Terraria 安装目录> --out <仓库外目录>
terraria extract --path <正版 Terraria 安装目录> --out <仓库外目录>
```
