# Terraria（Rust rewrite）

用 Rust + Spark 运行时重写的 **Terraria** 客户端实验仓。

**本仓库不是引擎。** Spark 是引擎，本仓是游戏。

运行前你必须拥有并安装正版 Terraria（Steam / GOG 等均可）。

- `--path` 必须指向 **正版安装根目录**（该目录下要有 `Content/`、`Content/Images/`）。
- 本仓库 **不附带**、也 **不重新分发**任何正版商业素材。
- 没有原版安装目录时，CLI 会直接失败，不会静默降级。

## 本机启动

```bash
pnpm install
pnpm launch
```

`pnpm launch` 会编译当前 Rust 与 TypeScript，再从本机 Steam 库找到正版安装根，并调用：

```text
terraria emulate --path <安装根>
```

安装不在默认库时：

```bash
pnpm launch -- --path "<正版安装根>"
```

发布构建：`pnpm launch -- --release`。

环境变量：

| 变量             | 含义                                 |
|------------------|--------------------------------------|
| `TR_CONTENT`     | 由 `emulate` 自动写入，指向 `--path` |
| `TR_NATIVE_NODE` | 可选，强制指定 `.node` 路径          |

## 素材命令

窗口以外还可以：

```bash
terraria unpack --path <正版安装根> --out <仓库外目录>
terraria extract --path <正版安装根> --out <仓库外目录> [--only Tiles_0.xnb]
```

`extract` 的 PNG 由 Spark 像素图写出。输出目录不能在本仓库或正版安装目录内。


## 开发构建

Spark 依赖根 `Cargo.toml` 的 git `dev` 分支。本机已有检出时，把 `.cargo/config.toml.example` 复制为 `.cargo/config.toml`（不入库），用 path 覆盖。不要改 `Cargo.toml`。

```bash
cargo check -p tr-napi
```

