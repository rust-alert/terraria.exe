# `@game-gpt/terraria`

Terraria Rust rewrite 的 npm 元包。

## 启动

```bash
terraria emulate --path <Terraria 安装根>
```

`--path` 必须是 Terraria 安装根（含 `Content/`）。没有安装根无法运行。不要使用 `cargo run`。游戏窗口由本包经 `tr-napi` 拉起。

## 素材

PNG 读写走 Spark 的像素图，不在本仓另写编码器。

```bash
terraria unpack --path <安装根> --out <仓库外目录>
terraria extract --path <安装根> --out <仓库外目录>
```

`unpack` 写出未压缩 XNB。`extract` 把贴图写成 PNG。`--only` 可重复，用来按路径子串筛选。`--out` 不能落在本仓库或 Terraria 安装目录里。解出的文件不要提交。
