import { invoke } from "@tauri-apps/api/core";

export type LoaderId = "neoforge" | "fabric";

export interface LoaderMeta {
  id: LoaderId;
  name: string;
  desc: string;
}

export const SUPPORTED_LOADERS: LoaderMeta[] = [
  {
    id: "neoforge",
    name: "NeoForge",
    desc: "面向新版 Minecraft 的 Forge 继任者",
  },
  {
    id: "fabric",
    name: "Fabric",
    desc: "轻量、模块化，兼容性极佳的加载器",
  },
];

/** 加载器版本目录条目（与后端 loader::LoaderVersion 对应） */
export interface LoaderVersion {
  version: string;
  stable: boolean;
}

/** 某 Minecraft 版本可用的加载器版本列表（后端按新到旧排列） */
export function list_loader_versions(
  kind: LoaderId,
  minecraftVersion: string,
): Promise<LoaderVersion[]> {
  return invoke<LoaderVersion[]>("list_loader_versions", {
    kind,
    minecraftVersion,
  });
}
