/**
 * 运行时表单 Schema：TS 的 interface 在运行时会被擦除，无法像 Rust 属性注解那样
 * 携带字段名/介绍/控件类型，因此用一份显式声明的对象来描述"哪个字段用什么控件、
 * 显示什么文案"，由 SchemaFormCard 据此自动生成表单 UI。
 *
 * 为了让 schema 同时作为"值的单一事实来源"，提供 FormValues 工具类型：
 * 只要 schema 用 `as const` 书写，就能自动推导出表单值对象的 TS 类型，
 * 字段增删/改类型时类型同步变化，避免 UI 定义与数据结构脱节。
 */

export interface EnumOption {
  /** 提交/持久化时使用的原始值 */
  value: string;
  /** 下拉框展示文案 */
  label: string;
}

export interface TextFieldSpec {
  kind: "text";
  /** 字段名，须与持久化结构（如后端 camelCase 字段）一致 */
  key: string;
  label: string;
  desc?: string;
  placeholder?: string;
  /** 是否占满整行（否则放入两列网格） */
  full?: boolean;
  required?: boolean;
  default?: string;
  /**
   * 可选的自定义同步校验：在内置校验（如必填）通过后调用，返回错误文案或 null。
   * 适用于无法用简单规则表达、且依赖外部状态的字段（如实例名称查重、目录名合法性）。
   */
  validate?: (value: string) => string | null;
}

/** 各控件 spec 共有的布局字段 */
export interface FieldLayout {
  /** 是否占满整行（否则放入两列网格） */
  full?: boolean;
}

export interface NumberFieldSpec {
  kind: "number";
  key: string;
  label: string;
  desc?: string;
  placeholder?: string;
  full?: boolean;
  required?: boolean;
  /** 仅允许输入整数 */
  integer?: boolean;
  min?: number;
  max?: number;
  /** 显示在输入框后的单位，如 px / MB */
  suffix?: string;
  /** 空值语义：null 表示"未填写"（交给全局默认托底） */
  default?: number | null;
}

/** 空格/逗号/分号分隔的字符串列表，编辑体验同 TestVersionDownload（JVM/游戏参数等） */
export interface ArgsFieldSpec {
  kind: "args";
  key: string;
  label: string;
  desc?: string;
  placeholder?: string;
  full?: boolean;
  required?: boolean;
  default?: string[];
}

export interface BooleanFieldSpec {
  kind: "boolean";
  key: string;
  label: string;
  desc?: string;
  full?: boolean;
  default?: boolean;
}

export interface EnumFieldSpec {
  kind: "enum";
  key: string;
  label: string;
  desc?: string;
  full?: boolean;
  required?: boolean;
  options: EnumOption[];
  default?: string;
}

export type FieldSpec =
  | TextFieldSpec
  | NumberFieldSpec
  | ArgsFieldSpec
  | BooleanFieldSpec
  | EnumFieldSpec;

export interface FormSection {
  title: string;
  desc?: string;
  fields: readonly FieldSpec[];
}

export interface FormSchema {
  sections: readonly FormSection[];
}

/** 根据某个字段 spec 推导其值的 TS 类型 */
export type FieldValue<F extends FieldSpec> = F extends
  | { kind: "text" }
  | { kind: "enum" }
  ? string
  : F extends { kind: "number" }
    ? number | null
    : F extends { kind: "boolean" }
      ? boolean
      : F extends { kind: "args" }
        ? string[]
        : never;

type ValuesFromFields<
  T extends readonly FieldSpec[],
  Acc = unknown,
> = T extends readonly [
  infer F extends FieldSpec,
  ...infer Rest extends readonly FieldSpec[],
]
  ? ValuesFromFields<Rest, Acc & { [P in F["key"]]: FieldValue<F> }>
  : Acc;

type ValuesFromSections<
  T extends readonly FormSection[],
  Acc = unknown,
> = T extends readonly [
  infer S extends FormSection,
  ...infer Rest extends readonly FormSection[],
]
  ? ValuesFromSections<Rest, Acc & ValuesFromFields<S["fields"]>>
  : Acc;

/** 从 schema 推导表单值对象类型：`FormValues<typeof SCHEMA>` */
export type FormValues<S extends FormSchema> = ValuesFromSections<
  S["sections"]
>;

/** 把空格/逗号/分号分隔的字符串切分为参数列表 */
export function split_args(text: string): string[] {
  return text
    .split(/[\s,，;；、]+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

/**
 * 校验单个字段。value 为当前值，raw 为控件内"原始文本"（number 用，用于保留
 * 未解析成功时的输入）；返回错误文案，null 表示通过。
 */
export function validate_field(
  field: FieldSpec,
  value: unknown,
  raw?: string,
): string | null {
  switch (field.kind) {
    case "text": {
      const text = typeof value === "string" ? value : "";
      if (field.required && text.trim().length === 0) {
        return `请填写「${field.label}」`;
      }
      if (field.validate) {
        return field.validate(text);
      }
      return null;
    }
    case "number": {
      const text = (raw ?? "").trim();
      if (text.length === 0) {
        return field.required ? `请填写「${field.label}」` : null;
      }
      const n = Number(text);
      if (!Number.isFinite(n)) {
        return `「${field.label}」必须是数字`;
      }
      if (field.integer && !Number.isInteger(n)) {
        return `「${field.label}」必须是整数`;
      }
      if (field.min !== undefined && n < field.min) {
        return `「${field.label}」不能小于 ${field.min}`;
      }
      if (field.max !== undefined && n > field.max) {
        return `「${field.label}」不能大于 ${field.max}`;
      }
      return null;
    }
    case "args": {
      if (!field.required) return null;
      if (!Array.isArray(value) || value.length === 0) {
        return `请为「${field.label}」至少填写一个参数`;
      }
      return null;
    }
    case "enum": {
      if (!field.required) return null;
      if (!(typeof value === "string" && value.length > 0)) {
        return `请选择「${field.label}」`;
      }
      return null;
    }
    case "boolean":
      return null;
  }
}

/** schema 中全部字段的扁平列表（用于遍历校验/初始化） */
export function flatten_fields(schema: FormSchema): readonly FieldSpec[] {
  return schema.sections.flatMap((s) => s.fields as FieldSpec[]);
}

/** 取字段的默认值 */
export function field_default(field: FieldSpec): unknown {
  switch (field.kind) {
    case "text":
      return field.default ?? "";
    case "number":
      return field.default !== undefined ? field.default : null;
    case "args":
      return field.default ?? [];
    case "boolean":
      return field.default ?? false;
    case "enum":
      return field.default ?? "";
  }
}
