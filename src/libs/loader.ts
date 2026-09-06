export type LoaderId = "neoforge" | "fabric" | "quilt";

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
