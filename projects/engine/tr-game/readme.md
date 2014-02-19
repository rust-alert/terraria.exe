# `tr-game`

Terraria 游戏逻辑库。仅由 `tr-napi` 拉起，不提供独立二进制入口。

不要使用 `cargo run`。游戏窗口由 npm CLI 的 `emulate` 拉起。`unpack` / `extract` 不打开窗口。

```bash
terraria emulate --path <Terraria 安装根>
terraria unpack --path <Terraria 安装根> --out <仓库外目录>
terraria extract --path <Terraria 安装根> --out <仓库外目录>
```
