# `tr-wasm`

Terraria Wasm 绑定。导出 C ABI，产物拷至 `projects/platforms/wasm/terraria-unknown-wasm32`。

```bash
cargo build -p tr-wasm --target wasm32-unknown-unknown --release
```

桌面游玩仍经 `@game-gpt/terraria` → `tr-napi`。
