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

// ========== 版本比较（用于加载器支持范围判定） ==========

/**
 * 解析正式版号如 1.20.1 / 1.21 / 26.2 为 (major, minor, patch)。
 * 含字母的快照（24w14a、1.21.5-pre1）等返回 null。
 */
export function mc_release_parts(
  id: string,
): [major: number, minor: number, patch: number] | null {
  const s = String(id);
  if (s.length === 0 || /[^0-9.]/.test(s)) return null;
  const nums = s
    .split(".")
    .filter((seg) => seg.length > 0)
    .map(Number);
  if (nums.length === 0 || nums.some((n) => !Number.isInteger(n))) return null;
  return [nums[0]!, nums[1] ?? 0, nums[2] ?? 0];
}

/** 正式版语义比较：a >= b */
export function mc_release_at_least(
  id: string,
  target: [number, number, number],
): boolean {
  const a = mc_release_parts(id);
  if (!a) return false;
  for (let i = 0; i < 3; i++) {
    if (a[i] > target[i]) return true;
    if (a[i] < target[i]) return false;
  }
  return true;
}

/**
 * NeoForge 是否支持该 Minecraft 版本：需要 1.20.1 及以上的正式版。
 * 快照 / 愚人节版本等一律视为不支持。
 */
export function neo_forge_supported(v: {
  id: string;
  type?: string;
}): boolean {
  if (v.type && v.type !== "release") return false;
  return mc_release_at_least(String(v.id), [1, 20, 1]);
}

export interface VersionPage {
    latest: LatestVersion,
    versions: SingleVersion[],
    page: number,
    size: number,
    total: number,
    totalPages: number,
    hasMore: boolean
}

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