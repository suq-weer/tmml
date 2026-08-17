export interface VersionManifest {
    latest: LatestVersion,
    versions: SingleVersion[]
}

export interface LatestVersion {
    release: String,
    snapshot: String,
}

export interface SingleVersion {
    id: String,
    type: String,
    url: String,
    time: String,
    releaseTime: String
}

export type VersionMode = "ALL" | "RELEASE" | "SNAPSHOT" | "FOOL";
