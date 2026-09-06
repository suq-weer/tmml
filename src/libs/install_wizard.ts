import type { SingleVersion } from "./mc_version";

export interface InstallStepMeta {
  path: string;
  label: string;
}

export const INSTALL_STEPS: InstallStepMeta[] = [
  { path: "/install/env", label: "环境配置" },
  { path: "/install/customize", label: "个性化" },
  { path: "/install/download", label: "开始下载" },
];

/** 任务已提交页：不是可选的向导步骤，而是 Stepper 的"全部完成"态 */
export const INSTALL_SUBMITTED_PATH = "/install/submitted";

export function install_step_index(routePath: string): number {
  const index = INSTALL_STEPS.findIndex((s) => s.path === routePath);
  return index < 0 ? 0 : index;
}

/** 将 SingleVersion 编码进路由 query，随 /install 子路由传递 */
export function single_version_to_query(v: SingleVersion): Record<string, string> {
  return {
    id: String(v.id),
    type: String(v.type),
    url: String(v.url),
    time: String(v.time),
    releaseTime: String(v.releaseTime),
  };
}

export function single_version_from_query(
  query: Record<string, unknown>,
): SingleVersion | null {
  const str = (key: string): string => {
    const value = query[key];
    if (typeof value === "string") return value;
    if (Array.isArray(value) && typeof value[0] === "string") return value[0];
    return "";
  };
  const id = str("id");
  if (!id) return null;
  return {
    id,
    type: str("type"),
    url: str("url"),
    time: str("time"),
    releaseTime: str("releaseTime"),
  };
}
