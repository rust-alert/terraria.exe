#!/usr/bin/env node
/**
 * 产品入口：
 *
 *   terraria emulate --path <正版 Terraria 安装目录>
 *   terraria unpack  --path <安装根> --out <目录> [--only <子串>]
 *   terraria extract --path <安装根> --out <目录> [--only <子串>]
 *
 * 窗口只由 emulate 打开。unpack / extract 不把文件写进本仓库。
 */

import { loadTerraria } from "./index";

function usage(): never {
    console.error(`用法：
  terraria emulate --path <正版 Terraria 安装目录>
  terraria unpack  --path <安装根> --out <目录> [--only <子串>]
  terraria extract --path <安装根> --out <目录> [--only <子串>]

必须传入已购买并安装的原版根目录（含 Content/）。
本仓库不附带商业素材。unpack / extract 的 --out 必须在仓库和安装目录之外。
不能用 cargo run 启动游戏。`);
    process.exitCode = 2;
    throw new Error("usage");
}

function takeValue(argv: string[], i: number, flag: string): string {
    const value = argv[i];
    if (!value || value.startsWith("--")) {
        console.error(`${flag} 缺少值。`);
        usage();
    }
    return value;
}

function parsePath(argv: string[]): { path: string; out?: string; only: string[] } {
    let original: string | undefined;
    let out: string | undefined;
    const only: string[] = [];
    for (let i = 0; i < argv.length; i++) {
        const a = argv[i];
        if (a === "--path" || a === "-p") {
            original = takeValue(argv, ++i, a);
            continue;
        }
        if (a.startsWith("--path=")) {
            original = a.slice("--path=".length);
            continue;
        }
        if (a === "--out" || a === "-o") {
            out = takeValue(argv, ++i, a);
            continue;
        }
        if (a.startsWith("--out=")) {
            out = a.slice("--out=".length);
            continue;
        }
        if (a === "--only") {
            only.push(takeValue(argv, ++i, a));
            continue;
        }
        if (a.startsWith("--only=")) {
            only.push(a.slice("--only=".length));
            continue;
        }
        console.error(`未知参数：${a}`);
        usage();
    }
    if (!original || !original.trim()) {
        console.error("缺少 --path。");
        usage();
    }
    return { path: original.trim(), out: out?.trim(), only };
}

function main() {
    const [command, ...rest] = process.argv.slice(2);
    if (command !== "emulate" && command !== "unpack" && command !== "extract") {
        usage();
    }
    const { path, out, only } = parsePath(rest);
    const host = loadTerraria();
    host.validatePath(path);
    if (command === "emulate") {
        if (out || only.length > 0) {
            console.error("emulate 不接受 --out / --only。");
            usage();
        }
        console.log(`terraria emulate：正版路径 ${path}`);
        host.emulate(path);
        return;
    }
    if (!out) {
        console.error("缺少 --out。解出的文件不能写进仓库。");
        usage();
    }
    const line =
        command === "unpack"
            ? host.unpack(path, out, only)
            : host.extract(path, out, only);
    console.log(line);
}

main();
