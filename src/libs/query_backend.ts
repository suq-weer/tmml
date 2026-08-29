import { invoke } from "@tauri-apps/api/core";
import { VersionManifest, VersionMode } from "./mc_version";

export interface MainConfig {
    accentColor: number;
    downloadConcurrency: number;
    mirrorUrl: string | null;
    defaultWidth?: number;
    defaultHeight?: number;
    defaultJvmArgs?: string[];
    defaultGameArgs?: string[];
    defaultLaunchCommandPrefix?: string[];
    defaultLaunchCommandSuffix?: string[];
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

export function fetch_main_config(): Promise<MainConfig | null> {
    return invoke<MainConfig>("get_main_config")
        .then((cfg) => cfg)
        .catch((err) => {
            console.error(err);
            return null;
        });
}

export function save_main_config(partial: Partial<MainConfig>): Promise<MainConfig> {
    return invoke<MainConfig>("set_main_config", partial);
}
