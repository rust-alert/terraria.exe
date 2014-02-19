/**
 * `pnpm launch`：编译当前 Rust 与 TypeScript，再启动本机 Terraria。
 *
 *   pnpm launch
 *   pnpm launch -- --path <Terraria 安装根>
 *   pnpm launch -- --release
 */

import { execFileSync, spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(path.join(root, "package.json"));

function fail(message) {
    console.error(message);
    process.exit(1);
}

function run(command, args) {
    const result = spawnSync(command, args, {
        cwd: root,
        stdio: "inherit",
        windowsHide: false,
    });
    if (result.error) {
        fail(result.error.message);
    }
    if ((result.status ?? 1) !== 0) {
        process.exit(result.status ?? 1);
    }
}

function parseArgs(argv) {
    let install;
    let release = false;
    for (let i = 0; i < argv.length; i++) {
        const arg = argv[i];
        if (arg === "--") {
            continue;
        }
        if (arg === "--release") {
            release = true;
            continue;
        }
        if (arg === "--path" || arg === "-p") {
            install = argv[++i];
            continue;
        }
        if (arg.startsWith("--path=")) {
            install = arg.slice("--path=".length);
            continue;
        }
        fail(`未知参数：${arg}`);
    }
    if (install !== undefined && !install.trim()) {
        fail("`--path` 不能为空。");
    }
    return { install: install?.trim(), release };
}

function isTerrariaRoot(dir) {
    return existsSync(path.join(dir, "Content", "Images"));
}

function decodeReg(buffer) {
    const utf16 = buffer.toString("utf16le");
    if (utf16.includes("SteamPath")) {
        return utf16;
    }
    return buffer.toString("utf8");
}

function steamRoots() {
    const roots = [];
    if (process.platform === "win32") {
        try {
            const out = execFileSync(
                "reg",
                ["query", "HKCU\\Software\\Valve\\Steam", "/v", "SteamPath"],
                { windowsHide: true },
            );
            const text = decodeReg(out);
            const match = text.match(/SteamPath\s+REG_\w+\s+(\S.*)/);
            if (match) {
                roots.push(match[1].trim());
            }
        } catch {
            /* 未装 Steam 或没有注册表项 */
        }
        for (const key of ["ProgramFiles(x86)", "ProgramFiles"]) {
            const base = process.env[key];
            if (base) {
                roots.push(path.join(base, "Steam"));
            }
        }
    } else if (process.platform === "darwin") {
        const home = process.env.HOME;
        if (home) {
            roots.push(
                path.join(home, "Library", "Application Support", "Steam"),
            );
        }
    } else if (process.env.HOME) {
        roots.push(path.join(process.env.HOME, ".steam", "steam"));
        roots.push(path.join(process.env.HOME, ".local", "share", "Steam"));
    }
    return roots;
}

function libraryRoots(steamRoot) {
    const libs = [steamRoot];
    const vdbs = [
        path.join(steamRoot, "steamapps", "libraryfolders.vdf"),
        path.join(steamRoot, "config", "libraryfolders.vdf"),
    ];
    for (const vdf of vdbs) {
        if (!existsSync(vdf)) {
            continue;
        }
        const text = readFileSync(vdf, "utf8");
        for (const match of text.matchAll(/"path"\s+"([^"]+)"/g)) {
            libs.push(match[1].replace(/\\\\/g, "\\"));
        }
    }
    return libs;
}

function discoverTerraria() {
    const seen = new Set();
    for (const steam of steamRoots()) {
        for (const lib of libraryRoots(steam)) {
            const install = path.join(lib, "steamapps", "common", "Terraria");
            const key = path.resolve(install).toLowerCase();
            if (seen.has(key)) {
                continue;
            }
            seen.add(key);
            if (isTerrariaRoot(install)) {
                return path.resolve(install);
            }
        }
    }
    return null;
}

function resolveTypescript() {
    let pkgJson;
    try {
        pkgJson = require.resolve("typescript/package.json");
    } catch {
        fail("未安装 typescript。请在仓库根执行 pnpm install。");
    }
    const pkg = JSON.parse(readFileSync(pkgJson, "utf8"));
    const bin = typeof pkg.bin === "string" ? pkg.bin : pkg.bin?.tsc;
    if (!bin) {
        fail("typescript 包没有 tsc 可执行文件。");
    }
    return path.join(path.dirname(pkgJson), bin);
}

function main() {
    const { install: given, release } = parseArgs(process.argv.slice(2));
    const install = given ?? discoverTerraria();
    if (!install) {
        fail(
            "没有找到本机 Terraria（Content/Images）。请安装 Steam 版，或执行：pnpm launch -- --path <安装根>",
        );
    }
    if (!isTerrariaRoot(install)) {
        fail(`不是 Terraria 安装根（缺少 Content/Images）：${install}`);
    }

    console.log(`pnpm launch：安装根 ${install}`);
    const napiArgs = [path.join(root, "scripts", "build", "napi.mjs")];
    if (release) {
        napiArgs.push("--release");
    }
    run(process.execPath, napiArgs);
    run(process.execPath, [
        resolveTypescript(),
        "-p",
        path.join(root, "projects", "hosts", "terraria", "tsconfig.json"),
    ]);
    run(process.execPath, [
        path.join(root, "projects", "hosts", "terraria", "dist", "cli.js"),
        "emulate",
        "--path",
        install,
    ]);
}

main();
