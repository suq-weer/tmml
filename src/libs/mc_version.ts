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

// ========== version.json 解析 ==========

export interface VersionContent {
  arguments: Arguments;
  assetsIndex: AssetsIndex;
  assets: string;
  complianceLevel: number;
  downloads: Downloads;
  id: string;
  javaVersion: JavaVersion;
  libraries: OnceLibraries[];
  logging: Logging;
  mainClass: string;
  minimumLauncherVersion: number;
  releaseTime: string;
  time: string;
  type: string;
}

// - arguments

export interface Arguments {
  game: (string | Argument)[];
  jvm: (string | Argument)[];
}

export interface Argument {
  rules: Rule[];
  value: string | string[];
}

export interface Rule {
  action: string;
  features?: FeaturesFlag;
  os?: OS;
}

export interface OS {
  name?: string;
  arch?: string;
}

export interface FeaturesFlag {
  is_demo_user?: boolean;
  has_custom_resolution?: boolean;
  has_quick_plays_support?: boolean;
  is_quick_play_singleplayer?: boolean;
  is_quick_play_multiplayer?: boolean;
  is_quick_play_realms?: boolean;
}

// - assetsIndex

export interface AssetsIndex {
  id: string;
  sha1: string;
  size: number;
  totalSize: number;
  url: string;
}

// - downloads

export interface Downloads {
  client?: DownloadsFile;
  client_mappings?: DownloadsFile;
  server?: DownloadsFile;
  server_mappings?: DownloadsFile;
}

export interface DownloadsFile {
  sha1: string;
  size: number;
  url: string;
}

// - javaVersion

export interface JavaVersion {
  component: string;
  majorVersion: number;
}

// - libraries

export interface OnceLibraries {
  rules?: Rule[];
  downloads: LibrariesDownloads;
  name: string;
}

export interface LibrariesDownloads {
  artifact: Artifact;
}

export interface Artifact {
  path: string;
  sha1: string;
  size: number;
  url: string;
}

// - logging

export interface Logging {
  client: LoggingClient;
}

export interface LoggingClient {
  argument: string;
  file: LoggingClientFile;
  type: string;
}

export interface LoggingClientFile {
  id: string;
  sha1: string;
  size: number;
  url: string;
}