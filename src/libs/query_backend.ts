import { invoke } from "@tauri-apps/api/core";
import { VersionManifest, VersionMode } from "./mc_version";

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
