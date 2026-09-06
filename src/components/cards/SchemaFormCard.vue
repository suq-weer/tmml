<script setup lang="ts">
import { reactive } from "vue";
import {
  type EnumFieldSpec,
  type FieldSpec,
  type FormSchema,
  type NumberFieldSpec,
  type TextFieldSpec,
  field_default,
  flatten_fields,
  split_args,
  validate_field,
} from "../../libs/form_schema";
import "@mdui/icons/file-open.js";

const props = withDefaults(
  defineProps<{
    schema: FormSchema;
    title?: string;
    subtitle?: string;
    /** 初始值（只用于首次渲染，之后以卡片内部编辑为准） */
    modelValue?: Record<string, unknown>;
  }>(),
  { title: "", subtitle: "", modelValue: () => ({}) },
);

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, unknown>): void;
}>();

const values = reactive<Record<string, unknown>>({});
/** number / args 控件内部保留"正在编辑的原文"，避免输入过程被值类型转换打断 */
const raw_text = reactive<Record<string, string>>({});
const errors = reactive<Record<string, string>>({});

const initial = props.modelValue ?? {};
for (const field of flatten_fields(props.schema)) {
  const key = field.key;
  values[key] =
    initial[key] !== undefined ? initial[key] : field_default(field);
  if (field.kind === "number") {
    raw_text[key] = typeof values[key] === "number" ? String(values[key]) : "";
  } else if (field.kind === "args") {
    raw_text[key] = Array.isArray(values[key])
      ? (values[key] as string[]).join(" ")
      : "";
  }
}

function commit(): void {
  emit("update:modelValue", { ...values });
}

function clearError(key: string): void {
  if (errors[key]) delete errors[key];
}

function asNumberField(f: FieldSpec): NumberFieldSpec {
  return f as NumberFieldSpec;
}
function asEnumField(f: FieldSpec): EnumFieldSpec {
  return f as EnumFieldSpec;
}
function asTextField(f: FieldSpec): TextFieldSpec {
  return f as TextFieldSpec;
}

function onTextInput(field: FieldSpec, e: Event): void {
  const key = field.key;
  values[key] = String((e.target as HTMLInputElement).value ?? "");
  clearError(key);
  commit();
  // 带自定义校验的字段在输入时即时回显（如实例名称查重/合法性），无需等到提交
  if (field.kind === "text" && asTextField(field).validate) {
    const msg = validate_field(field, values[key]);
    if (msg) errors[key] = msg;
  }
}

function onNumberInput(field: FieldSpec, e: Event): void {
  const key = field.key;
  const raw = String((e.target as HTMLInputElement).value ?? "");
  raw_text[key] = raw;
  const trimmed = raw.trim();
  const n = trimmed.length > 0 ? Number(trimmed) : NaN;
  values[key] = trimmed.length > 0 && Number.isFinite(n) ? n : null;
  clearError(key);
  commit();
}

function onArgsInput(field: FieldSpec, e: Event): void {
  const key = field.key;
  const raw = String((e.target as HTMLInputElement).value ?? "");
  raw_text[key] = raw;
  values[key] = split_args(raw);
  clearError(key);
  commit();
}

function onBoolToggle(field: FieldSpec, e: Event): void {
  const key = field.key;
  values[key] = Boolean((e.target as HTMLInputElement).checked);
  clearError(key);
  commit();
}

function onEnumChange(field: FieldSpec, e: Event): void {
  const key = field.key;
  values[key] = String((e.target as HTMLInputElement).value ?? "");
  clearError(key);
  commit();
}

function args_token_count(field: FieldSpec): number {
  const raw = raw_text[field.key] ?? "";
  return raw.trim().length === 0 ? 0 : split_args(raw).length;
}

/** 校验整个表单：任一字段不合法则记录错误并返回 false */
function validate(): boolean {
  for (const key of Object.keys(errors)) delete errors[key];
  let invalid = false;
  for (const field of flatten_fields(props.schema)) {
    const msg =
      field.kind === "number"
        ? validate_field(field, values[field.key], raw_text[field.key])
        : validate_field(field, values[field.key]);
    if (msg) {
      errors[field.key] = msg;
      invalid = true;
    }
  }
  return !invalid;
}

/** 返回第一条错误文案（配合 validate() 使用，供父容器提醒用户） */
function firstError(): string | null {
  for (const field of flatten_fields(props.schema)) {
    if (errors[field.key]) return errors[field.key];
  }
  return null;
}

function reset(): void {
  for (const field of flatten_fields(props.schema)) {
    const key = field.key;
    const def = field_default(field);
    values[key] = def;
    if (field.kind === "number") {
      raw_text[key] = typeof def === "number" ? String(def) : "";
    } else if (field.kind === "args") {
      raw_text[key] = Array.isArray(def) ? (def as string[]).join(" ") : "";
    }
    if (errors[key]) delete errors[key];
  }
  commit();
}

defineExpose({ validate, firstError, reset, errors });
</script>

<template>
  <mdui-card variant="outlined" class="schema-card">
    <header v-if="props.title" class="card-head">
      <mdui-icon-file-open class="head-icon"></mdui-icon-file-open>
      <div>
        <div class="card-title">{{ props.title }}</div>
        <div v-if="props.subtitle" class="card-sub">
          {{ props.subtitle }}
        </div>
      </div>
    </header>

    <template
      v-for="(section, sectionIndex) in props.schema.sections"
      :key="sectionIndex"
    >
      <mdui-divider v-if="sectionIndex > 0 || props.title"></mdui-divider>
      <div v-if="section.title" class="section-head">
        <p class="section-title">{{ section.title }}</p>
        <sub v-if="section.desc" class="section-desc">{{ section.desc }}</sub>
      </div>

      <div class="grid">
        <!-- boolean：整行开关 -->
        <template v-for="field in section.fields" :key="field.key">
          <div
            v-if="field.kind === 'boolean'"
            class="bool-row"
            :class="{ 'has-error': !!errors[field.key] }"
          >
            <div class="bool-text">
              <p class="field-label">{{ field.label }}</p>
              <sub v-if="field.desc" class="field-desc">{{ field.desc }}</sub>
              <div v-if="errors[field.key]" class="support error">
                {{ errors[field.key] }}
              </div>
            </div>
            <mdui-switch
              :checked="Boolean(values[field.key])"
              @change="onBoolToggle(field, $event)"
            ></mdui-switch>
          </div>

          <!-- text / number / enum：一格 -->
          <div
            v-else
            class="field"
            :class="{
              full:
                field.full === true ||
                field.kind === 'args' ||
                field.kind === 'text',
            }"
          >
            <template v-if="field.kind === 'text'">
              <mdui-text-field
                variant="outlined"
                :label="field.label"
                :placeholder="field.placeholder"
                :value="String(values[field.key] ?? '')"
                :clearable="true"
                @input="onTextInput(field, $event)"
              ></mdui-text-field>
            </template>

            <template v-else-if="field.kind === 'number'">
              <mdui-text-field
                variant="outlined"
                type="number"
                :label="field.label"
                :placeholder="field.placeholder"
                :suffix="asNumberField(field).suffix"
                :value="raw_text[field.key] ?? ''"
                @input="onNumberInput(field, $event)"
              ></mdui-text-field>
            </template>

            <template v-else-if="field.kind === 'enum'">
              <mdui-select
                variant="outlined"
                :label="field.label"
                :value="String(values[field.key] ?? '')"
                @change="onEnumChange(field, $event)"
              >
                <mdui-menu-item
                  v-for="option in asEnumField(field).options"
                  :key="option.value"
                  :value="option.value"
                  >{{ option.label }}</mdui-menu-item
                >
              </mdui-select>
            </template>

            <template v-else-if="field.kind === 'args'">
              <mdui-text-field
                variant="outlined"
                :label="field.label"
                :placeholder="field.placeholder"
                :value="raw_text[field.key] ?? ''"
                @input="onArgsInput(field, $event)"
              ></mdui-text-field>
            </template>

            <div class="support-line">
              <span v-if="field.desc" class="support">{{ field.desc }}</span>
              <span
                v-if="field.kind === 'args' && args_token_count(field) > 0"
                class="support tokens"
                >已解析 {{ args_token_count(field) }} 个参数</span
              >
              <span v-if="errors[field.key]" class="support error">{{
                errors[field.key]
              }}</span>
            </div>
          </div>
        </template>
      </div>
    </template>
  </mdui-card>
</template>

<style scoped>
.schema-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.25rem 0.9rem;
}

.head-icon {
  font-size: 1.75rem;
  color: rgb(var(--mdui-color-primary));
}

.card-title {
  font-size: var(--mdui-typescale-title-medium-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
}

.card-sub {
  margin-top: 0.15rem;
  font-size: 0.8125rem;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.section-head {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
  padding: 1rem 1.25rem 0.25rem;
  flex-wrap: wrap;
}

.section-title {
  margin: 0;
  font-size: var(--mdui-typescale-title-small-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
  color: rgb(var(--mdui-color-primary));
}

.section-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
  gap: 0.1rem 1rem;
  padding: 0.25rem 1.25rem 1rem;
  align-items: start;
}

.field {
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding-top: 0.15rem;
}

.field.full {
  grid-column: 1 / -1;
}

.field mdui-text-field,
.field mdui-select {
  width: 100%;
}

.support-line {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-height: 1.05rem;
  padding: 2px 0 6px;
}

.support {
  font-size: 0.75rem;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.support.tokens {
  color: rgb(var(--mdui-color-primary));
}

.support.error {
  color: rgb(var(--mdui-color-error));
}

.bool-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  grid-column: 1 / -1;
  padding: 0.35rem 0.1rem;
}

.bool-row + .bool-row {
  border-top: 1px solid rgb(var(--mdui-color-outline-variant));
}

.bool-row + .field,
.field + .bool-row {
  margin-top: 0.4rem;
}

.bool-text {
  flex: 1;
  min-width: 0;
  line-height: 1.3;
}

.field-label {
  margin: 0;
  font-size: var(--mdui-typescale-body-large-size);
}

.field-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.bool-row.has-error .field-label {
  color: rgb(var(--mdui-color-error));
}
</style>
