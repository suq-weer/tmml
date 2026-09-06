import type { InstanceConfig } from "./instance";
import {
  type FormSchema,
  type FormValues,
  field_default,
  flatten_fields,
} from "./form_schema";

// | 实例名称校验：目录名合法性（Windows + Linux）+ 本地查重 |

/** 与后端 instance::sanitize_dir_name 剔除的字符保持一致，保证最终目录名 === 输入的名称 */
const FOLDER_INVALID_CHARS = /[<>:"/\\|?*\u0000-\u001f\u007f]/;

/** Windows 保留设备名（按第一个点之前的部分判定，如 CON、CON.txt、COM¹，大小写不敏感） */
const WINDOWS_RESERVED = new Set([
  "con",
  "prn",
  "aux",
  "nul",
  "clock$",
  "conin$",
  "conout$",
  "com0",
  "com1",
  "com2",
  "com3",
  "com4",
  "com5",
  "com6",
  "com7",
  "com8",
  "com9",
  "lpt0",
  "lpt1",
  "lpt2",
  "lpt3",
  "lpt4",
  "lpt5",
  "lpt6",
  "lpt7",
  "lpt8",
  "lpt9",
  "com¹",
  "com²",
  "com³",
  "lpt¹",
  "lpt²",
  "lpt³",
]);

/** 目录名最大长度（Windows 单个路径分量上限为 255，这里留出游戏文件的余量） */
const MAX_INSTANCE_NAME_LEN = 80;

interface InstanceNameContext {
  /** 进入向导的版本 ID：实例名留空时以其作为回退目录名 */
  versionId: string;
  /** 本地已存在实例的目录名（小写），用于查重 */
  folderKeys: Set<string>;
}

let instance_name_ctx: InstanceNameContext = {
  versionId: "",
  folderKeys: new Set(),
};

/** 从注册表 path（形如 `versions/<dir>`）中提取目录名 */
function instance_dir_name(path: string): string {
  const tail = path.split("/").pop() ?? "";
  return tail.length > 0 && !tail.includes("\\") ? tail : "";
}

/** 提供校验所需上下文：进入向导的版本（空名回退名称）与本地实例列表 */
export function set_instance_name_context(
  versionId: string,
  instances: readonly { path: string }[],
): void {
  instance_name_ctx.versionId = versionId;
  instance_name_ctx.folderKeys = new Set(
    instances
      .map((i) => instance_dir_name(i.path))
      .filter((s) => s.length > 0)
      .map((s) => s.toLowerCase()),
  );
}

/** 文件夹名在 Windows 与 Linux 上均合法的检查；返回错误文案，合法返回 null */
export function folder_name_error(raw: string): string | null {
  const name = raw.trim();
  if (name.length === 0 || name === "." || name === "..") {
    return "实例名称不能为空，且不能是 . 或 ..";
  }
  if (name.length > MAX_INSTANCE_NAME_LEN) {
    return `实例名称过长（最多 ${MAX_INSTANCE_NAME_LEN} 个字符）`;
  }
  if (FOLDER_INVALID_CHARS.test(name)) {
    return '实例名称含非法字符，无法用作文件夹名：< > : " / \\ | ? * 或控制字符';
  }
  if (name.endsWith(".") || name.endsWith(" ")) {
    return "实例名称不能以点或空格结尾（Windows 不允许）";
  }
  const base = name.split(".")[0]!.toLowerCase();
  if (WINDOWS_RESERVED.has(base)) {
    return `「${base}」是 Windows 保留设备名，不能用作文件夹名`;
  }
  return null;
}

/**
 * “实例名称”字段的自定义校验：名称会作为实例目录名，故先校验其在 Windows / Linux
 * 上是否合法，再与本地已有实例查重（目录名冲突即会互相覆盖，需改不同名以共存）。
 * 留空时以版本 ID 作为回退名称参与同样的检查。
 */
export function validate_instance_name(raw: string): string | null {
  const typed = raw.trim();
  const candidate = typed.length > 0 ? typed : instance_name_ctx.versionId;
  const folderError = folder_name_error(candidate);
  if (folderError) return folderError;
  if (instance_name_ctx.folderKeys.has(candidate.toLowerCase())) {
    return `已有同名实例「${candidate}」，请换一个名称以便两实例共存`;
  }
  return null;
}

/**
 * "个性化设置"步骤的表单 schema。
 *
 * 字段与后端 Rust `InstanceConfig`（src-tauri/src/instance.rs）的 serde camelCase
 * 字段一一对应，这样向导最终提交时能直接交给下载任务，由后端在创建实例时
 * 持久化为本地 tmml_instance.json。schema 用 as const 书写，配合
 * FormValues 自动推导出 InstanceCustomizeValues 类型。
 */
export const INSTANCE_CUSTOMIZE_SCHEMA = {
  sections: [
    {
      title: "实例",
      fields: [
        {
          kind: "text",
          key: "name",
          label: "实例名称",
          desc: "留空则以版本 ID 命名；名称将作为实例目录名，需在本地唯一且对 Windows / Linux 均合法",
          placeholder: "例如：生存-1.21.1",
          full: true,
          validate: validate_instance_name,
        },
      ],
    },
    {
      title: "启动与运行",
      fields: [
        {
          kind: "text",
          key: "javaPath",
          label: "Java 可执行文件",
          desc: "留空则自动从 PATH 检测",
          placeholder: "例如：/usr/lib/jvm/java-21/bin/java",
          full: true,
        },
        {
          kind: "args",
          key: "jvmArgs",
          label: "JVM 参数",
          desc: "以空格/逗号/分号分隔；留空则使用全局默认 JVM 参数",
          placeholder: "-Xmx4G -XX:+UseG1GC",
          full: true,
        },
        {
          kind: "args",
          key: "launchCommandPrefix",
          label: "启动命令前缀",
          desc: "拼在 java 命令之前的命令（一般不需要填写）",
          placeholder: "例如：env LC_ALL=zh_CN.UTF-8",
          full: true,
        },
        {
          kind: "args",
          key: "launchCommandSuffix",
          label: "启动命令后缀",
          desc: "追加在游戏参数之后的命令（一般不需要填写）",
          full: true,
        },
      ],
    },
    {
      title: "显示",
      fields: [
        {
          kind: "number",
          key: "width",
          label: "窗口宽度",
          desc: "留空则使用全局默认分辨率",
          placeholder: "默认",
          integer: true,
          min: 1,
          suffix: "px",
        },
        {
          kind: "number",
          key: "height",
          label: "窗口高度",
          desc: "留空则使用全局默认分辨率",
          placeholder: "默认",
          integer: true,
          min: 1,
          suffix: "px",
        },
      ],
    },
    {
      title: "游戏",
      fields: [
        {
          kind: "args",
          key: "gameArgs",
          label: "游戏参数",
          desc: "以空格/逗号/分号分隔；留空则使用全局默认游戏参数",
          placeholder: "--quickPlaySingleplayer 世界名",
          full: true,
        },
      ],
    },
  ],
} as const satisfies FormSchema;

/** 由 schema 自动推导出的表单值类型 */
export type InstanceCustomizeValues = FormValues<
  typeof INSTANCE_CUSTOMIZE_SCHEMA
>;

export function defaults_customize_values(
  versionId: string,
): InstanceCustomizeValues {
  const values = {} as InstanceCustomizeValues;
  for (const field of flatten_fields(INSTANCE_CUSTOMIZE_SCHEMA)) {
    (values as Record<string, unknown>)[field.key] = field_default(field);
  }
  (values as Record<string, unknown>).name = versionId;
  return values;
}

/**
 * 向导跨步骤传递的临时状态：Customize 步骤填写，Start 步骤提交下载任务时读取。
 * key 为进入向导时的版本 ID，保证为不同版本开新向导不会互相污染。
 */
let cached_version = "";
let cached_values: InstanceCustomizeValues | null = null;

export function get_customize_values(versionId: string): InstanceCustomizeValues {
  if (!cached_values || cached_version !== versionId) {
    cached_values = defaults_customize_values(versionId);
    cached_version = versionId;
  }
  instance_name_ctx.versionId = versionId;
  return cached_values;
}

export function set_customize_values(values: InstanceCustomizeValues): void {
  cached_values = values;
}

/** 把表单值转换为可直接提交给下载任务的 InstanceConfig（空值交给后端托底） */
export function build_instance_config(
  values: InstanceCustomizeValues,
): InstanceConfig {
  const pick = (key: "jvmArgs" | "gameArgs" | "launchCommandPrefix" | "launchCommandSuffix") =>
    values[key];
  const java_path = values.javaPath.trim();
  return {
    launchCommandPrefix: pick("launchCommandPrefix"),
    launchCommandSuffix: pick("launchCommandSuffix"),
    javaPath: java_path.length > 0 ? java_path : null,
    jvmArgs: pick("jvmArgs"),
    gameArgs: pick("gameArgs"),
    width: values.width ?? undefined,
    height: values.height ?? undefined,
  };
}

/** 实例显示名：未填写则退回版本 ID */
export function instance_display_name(
  values: InstanceCustomizeValues,
  versionId: string,
): string {
  const name = values.name.trim();
  return name.length > 0 ? name : versionId;
}
