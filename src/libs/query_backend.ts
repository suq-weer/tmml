import { invoke } from "@tauri-apps/api/core";
import { VersionManifest, VersionMode } from "./mc_version";

export function info_f(text: string) {
    invoke('info_f', { text: text })
}

export function debug_f(text: string) {
    invoke('debug_f', { text: text })
}

export function warn_f(text: string) {
    invoke('warn_f', { text: text })
}

export async function fetch_mc_version_paged(size: number = 20, page: number = 1, version_mode: VersionMode = "SNAPSHOT"): Promise<VersionManifest | null> {
    return invoke<VersionManifest>(
        "get_minecraft_version",
        { size: size, page: page, versionMode: version_mode }
    ).then((ver) => {
        console.log(ver);
        return ver;
    }).catch((err) => {
        console.error(err);
        return null;
    });
}
