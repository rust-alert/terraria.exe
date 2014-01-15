import { createRequire } from "node:module";

import {
    detectNativePlatformPackage,
    listPlatformPackages,
    resolveNativePath,
    WASM_PLATFORM_PACKAGE,
    type TerrariaPlatformPackage,
} from "./platform";

export {
    detectNativePlatformPackage,
    listPlatformPackages,
    nativePackageName,
    platformShort,
    platformTriple,
    resolveNativePath,
    WASM_PLATFORM_PACKAGE,
    type PlatformInfo,
    type TerrariaNativeShort,
    type TerrariaPlatformPackage,
} from "./platform";

const nodeRequire = createRequire(__filename);

/** 与 `tr-napi`（`JsTerrariaHost`）对齐。 */
export interface TerrariaHostBindings {
    info(): { name: string; version: string; npmPackage: string };
    vec2Length(x: number, y: number): number;
    blockCount(): number;
    validatePath(path: string): void;
    /** 阻塞至窗口关闭。`path` 必须是正版 Terraria 安装根。 */
    emulate(path: string): void;
    /** 写出未压缩 XNB。`out` 必须在仓库和安装目录之外。 */
    unpack(path: string, out: string, only: string[]): string;
    /** 把 Texture2D 写成 PNG。编码走 spark-image。 */
    extract(path: string, out: string, only: string[]): string;
}

interface TerrariaAddon {
    JsTerrariaHost: new () => {
        info(): { name: string; version: string; npm_package: string };
        vec2Length(x: number, y: number): number;
        blockCount(): number;
        validatePath(path: string): void;
        emulate(path: string): void;
        unpack(path: string, out: string, only: string[]): string;
        extract(path: string, out: string, only: string[]): string;
    };
}

export interface LoadOptions {
    platformPackage?: TerrariaPlatformPackage;
}

let _cached: TerrariaHostBindings | undefined;

/**
 * 加载当前平台原生绑定。
 * 窗口启动只能走 `host.emulate(originalInstallPath)`。
 * `unpack` / `extract` 不打开窗口。
 */
export function loadTerraria(_options: LoadOptions = {}): TerrariaHostBindings {
    if (_cached) return _cached;
    if (_options.platformPackage === WASM_PLATFORM_PACKAGE) {
        throw new Error(
            "`loadTerraria` 只加载原生 `.node`。Wasm 请使用 `@game-gpt/terraria-unknown-wasm32`。",
        );
    }
    const addonPath = resolveNativePath();
    const addon = nodeRequire(addonPath) as TerrariaAddon;
    if (typeof addon.JsTerrariaHost !== "function") {
        throw new Error(
            `原生插件缺少 JsTerrariaHost：${addonPath}。请运行 node scripts/build/napi.mjs`,
        );
    }
    const host = new addon.JsTerrariaHost();
    _cached = {
        info: () => {
            const i = host.info();
            return {
                name: i.name,
                version: i.version,
                npmPackage: i.npm_package,
            };
        },
        vec2Length: (x, y) => host.vec2Length(x, y),
        blockCount: () => host.blockCount(),
        validatePath: (path) => host.validatePath(path),
        emulate: (path) => host.emulate(path),
        unpack: (path, out, only) => host.unpack(path, out, only),
        extract: (path, out, only) => host.extract(path, out, only),
    };
    return _cached;
}

export function currentNativePackage() {
    return detectNativePlatformPackage();
}
