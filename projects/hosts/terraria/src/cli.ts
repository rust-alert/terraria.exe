#!/usr/bin/env node
/**
 * 唯一产品入口：
 *
 *   terraria emulate --path <正版 Terraria 安装目录>
 *
 * 不提供其它启动子命令。
 */

import { loadTerraria } from "./index";

function usage(): never {
    console.error(`用法：
  terraria emulate --path <正版 Terraria 安装目录>

必须传入已购买并安装的原版根目录（含 Content/）。
本仓库不附带商业素材，也不能用 cargo run 启动游戏。`);
    process.exitCode = 2;
    throw new Error("usage");
}

function parseArgs(argv: string[]): { path: string } {
    if (argv[0] !== "emulate") {
        usage();
    }
    let original: string | undefined;
    for (let i = 1; i < argv.length; i++) {
        const a = argv[i];
        if (a === "--path" || a === "-p") {
            original = argv[++i];
            continue;
        }
        if (a.startsWith("--path=")) {
            original = a.slice("--path=".length);
            continue;
        }
        console.error(`未知参数：${a}`);
        usage();
    }
    if (!original || !original.trim()) {
        console.error("缺少 --path。");
        usage();
    }
    return { path: original.trim() };
}

function main() {
    const { path } = parseArgs(process.argv.slice(2));
    const host = loadTerraria();
    host.validatePath(path);
    console.log(`terraria emulate：正版路径 ${path}`);
    host.emulate(path);
}

main();
