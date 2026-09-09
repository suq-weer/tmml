import type { LoaderId } from "./loader";

/** 向导内选定的加载器（含版本与可选附装项），供提交页读取 */
export interface LoaderChoice {
  kind: LoaderId;
  version: string;
  withFabricApi: boolean;
}

interface Cache {
  minecraft: string;
  choice: LoaderChoice | null;
}

let cache: Cache = { minecraft: "", choice: null };

/** 记录「某 MC 版本」的加载器选择；传 null 表示不装加载器 */
export function set_loader_choice(
  minecraft: string,
  choice: LoaderChoice | null,
): void {
  cache = { minecraft, choice };
}

/** 读取进入向导的 MC 版本对应的加载器选择；未选/版本不匹配返回 null */
export function get_loader_choice(minecraft: string): LoaderChoice | null {
  return cache.minecraft === minecraft ? cache.choice : null;
}
