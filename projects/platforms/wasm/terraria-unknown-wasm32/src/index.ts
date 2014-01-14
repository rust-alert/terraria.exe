/**
 * `tr-unknown-wasm32`：Wasm 目标的 TypeScript 加载器。
 * 桌面游玩仍须使用 `@game-gpt/terraria` 的 `terraria emulate --path`。
 */

export const rustTarget = "wasm32-unknown-unknown" as const;
export const platformPackage = "tr-unknown-wasm32" as const;

export interface WasmInfo {
    name: string;
    version: string;
    npm_package: string;
    versionCode: number;
}

export interface TerrariaWasmBindings {
    info(): WasmInfo;
    vec2Length(x: number, y: number): number;
}

type WasmExports = {
    tr_vec2_length: (x: number, y: number) => number;
    tr_version_code: () => number;
};

function decodeVersion(code: number): string {
    const major = Math.floor(code / 1_000_000);
    const minor = Math.floor((code % 1_000_000) / 1_000);
    const patch = code % 1_000;
    return `${major}.${minor}.${patch}`;
}

export async function loadTerrariaWasm(): Promise<TerrariaWasmBindings> {
    const url = new URL("../tr_bg.wasm", import.meta.url);
    let exports: Partial<WasmExports> | undefined;
    try {
        if (typeof fetch === "function") {
            const resp = await fetch(url);
            if (resp.ok) {
                const buf = await resp.arrayBuffer();
                const { instance } = await WebAssembly.instantiate(buf, {});
                exports = instance.exports as unknown as WasmExports;
            }
        }
    } catch {
        /* 回退纯 JS */
    }

    if (exports?.tr_vec2_length && exports.tr_version_code) {
        const code = exports.tr_version_code();
        return {
            info: () => ({
                name: "Terraria",
                version: decodeVersion(code),
                npm_package: platformPackage,
                versionCode: code,
            }),
            vec2Length: (x, y) => exports!.tr_vec2_length!(x, y),
        };
    }

    return {
        info: () => ({
            name: "Terraria",
            version: "0.0.0",
            npm_package: platformPackage,
            versionCode: 0,
        }),
        vec2Length: (x, y) => Math.hypot(x, y),
    };
}
