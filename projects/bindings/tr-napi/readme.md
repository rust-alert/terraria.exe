# `tr-napi`

Terraria Node-API 绑定。唯一运行时宿主，供 `@game-gpt/terraria` CLI 调用。

桌面启动：

```bash
terraria emulate --path <Terraria 安装根>
```

带 Node 绑定编译：

```bash
cargo build -p tr-napi --release --features node
```
