<script lang="ts" setup>
import "@mdui/icons/extension--outlined.js";
import "@mdui/icons/settings--outlined.js";
import "@mdui/icons/code--outlined.js";
import "@mdui/icons/rocket--outlined.js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { VersionContent } from "../../libs/mc_version";
import { get_instance, InstanceInfo } from "../../libs/instance";
import { fetch_main_config, save_main_config } from "../../libs/query_backend";
import { onMounted, onUnmounted, ref } from "vue";

interface DownloadFinished {
  versionId: string;
  success: boolean;
  error: string | null;
}

// ---- 实例配置表单 ----
var version_id = ref("1.21.1");
var instance_name = ref("");
var width_str = ref("");
var height_str = ref("");
var jvm_str = ref("");
var game_str = ref("");
var prefix_str = ref("");
var suffix_str = ref("");

// ---- 全局设置 ----
var mirror_url = ref("");
var concurrency_str = ref("16");
var g_width_str = ref("800");
var g_height_str = ref("600");
var g_jvm_str = ref("");
var g_game_str = ref("");
var g_prefix_str = ref("");
var g_suffix_str = ref("");

// ---- 状态 ----
var downloading = ref(false);
var status = ref("尚未开始");
var finished = ref<DownloadFinished | null>(null);
var instance_info = ref<InstanceInfo | null>(null);
var test_text = ref("");
var open_sections = ref<string[]>(["global"]);

var unlisten_finished: UnlistenFn | null = null;

onMounted(async () => {
  const cfg = await fetch_main_config();
  if (cfg) {
    mirror_url.value = cfg.mirrorUrl ?? "";
    concurrency_str.value = String(cfg.downloadConcurrency ?? 16);
    // 旧版后端可能缺失新字段，做空值兜底
    const jvm = cfg.defaultJvmArgs ?? [];
    const game = cfg.defaultGameArgs ?? [];
    const pre = cfg.defaultLaunchCommandPrefix ?? [];
    const suf = cfg.defaultLaunchCommandSuffix ?? [];
    const dw = cfg.defaultWidth ?? 800;
    const dh = cfg.defaultHeight ?? 600;
    g_width_str.value = String(dw);
    g_height_str.value = String(dh);
    g_jvm_str.value = jvm.join(" ");
    g_game_str.value = game.join(" ");
    g_prefix_str.value = pre.join(" ");
    g_suffix_str.value = suf.join(" ");
    // 用全局默认托底预填实例表单
    width_str.value = String(dw);
    height_str.value = String(dh);
    jvm_str.value = jvm.join(" ");
    game_str.value = game.join(" ");
    prefix_str.value = pre.join(" ");
    suffix_str.value = suf.join(" ");
    status.value = "已加载全局默认配置";
  }

  unlisten_finished = await listen<DownloadFinished>(
    "minecraft-download-finished",
    (e) => {
      finished.value = e.payload;
      downloading.value = false;
      status.value = e.payload.success ? "实例创建完成" : "下载失败";
      if (e.payload.success) {
        get_instance(version_id.value)
          .then((info) => {
            instance_info.value = info;
          })
          .catch((err) => console.error(err));
      }
    },
  );
});

onUnmounted(() => {
  unlisten_finished?.();
});

function parse_args(text: string): string[] {
  return text
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function parse_opt_uint(text: string): number | undefined {
  const n = Number(text.trim());
  return Number.isFinite(n) && n >= 0 ? Math.trunc(n) : undefined;
}

function build_config() {
  const config: any = {
    launchCommandPrefix: parse_args(prefix_str.value),
    launchCommandSuffix: parse_args(suffix_str.value),
    jvmArgs: parse_args(jvm_str.value),
    gameArgs: parse_args(game_str.value),
  };
  const w = parse_opt_uint(width_str.value);
  const h = parse_opt_uint(height_str.value);
  if (w !== undefined) config.width = w;
  if (h !== undefined) config.height = h;
  return config;
}

function create_new_instance() {
  downloading.value = true;
  finished.value = null;
  instance_info.value = null;
  status.value = "开始创建实例 " + version_id.value + " ...";
  invoke("download_minecraft_version", {
    versionId: version_id.value,
    instanceName: instance_name.value.trim() || version_id.value,
    instanceConfig: build_config(),
  })
    .then(() => {
      status.value = "下载任务已提交完成";
    })
    .catch((e) => {
      status.value = "调用失败: " + e;
      downloading.value = false;
    });
}

function save_global_config() {
  save_main_config({
    downloadConcurrency: parse_opt_uint(concurrency_str.value) ?? 16,
    mirrorUrl: mirror_url.value.trim(),
    defaultWidth: parse_opt_uint(g_width_str.value) ?? 800,
    defaultHeight: parse_opt_uint(g_height_str.value) ?? 600,
    defaultJvmArgs: parse_args(g_jvm_str.value),
    defaultGameArgs: parse_args(g_game_str.value),
    defaultLaunchCommandPrefix: parse_args(g_prefix_str.value),
    defaultLaunchCommandSuffix: parse_args(g_suffix_str.value),
  })
    .then((cfg) => {
      status.value = "全局配置已保存";
      // 同步预填到实例表单
      const jvm = cfg.defaultJvmArgs ?? [];
      const game = cfg.defaultGameArgs ?? [];
      const pre = cfg.defaultLaunchCommandPrefix ?? [];
      const suf = cfg.defaultLaunchCommandSuffix ?? [];
      width_str.value = String(cfg.defaultWidth ?? 800);
      height_str.value = String(cfg.defaultHeight ?? 600);
      jvm_str.value = jvm.join(" ");
      game_str.value = game.join(" ");
      prefix_str.value = pre.join(" ");
      suffix_str.value = suf.join(" ");
    })
    .catch((e) => {
      status.value = "保存配置失败: " + e;
    });
}

function fetch_version_json() {
  invoke<VersionContent>("get_version", {})
    .then((v) => {
      console.log(v);
      test_text.value = JSON.stringify(v);
      open_sections.value = ["global", "raw"];
    })
    .catch((e) => {
      test_text.value = "获取失败: " + e;
    });
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div class="head-row">
        <mdui-button-icon class="arrow-back-button" @click="$router.back()">
          <mdui-icon-arrow-back></mdui-icon-arrow-back>
        </mdui-button-icon>
        <h2>新建实例</h2>
      </div>
      <p class="page-desc">
        选择版本并配置启动参数，下载完成后将自动建立实例。
      </p>
    </header>

    <mdui-card variant="outlined" class="card">
      <div class="card-head">
        <mdui-icon-extension--outlined
          class="card-icon"
        ></mdui-icon-extension--outlined>
        <div>
          <div class="card-title">实例配置</div>
          <div class="card-sub">留空的参数将以全局默认配置托底</div>
        </div>
      </div>
      <mdui-divider></mdui-divider>
      <div class="field-grid">
        <mdui-text-field
          label="版本号"
          :value="version_id"
          @input="version_id = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="实例名称（留空=版本号）"
          :value="instance_name"
          @input="instance_name = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="分辨率宽"
          type="number"
          :value="width_str"
          @input="width_str = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="分辨率高"
          type="number"
          :value="height_str"
          @input="height_str = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="JVM 参数（空格/逗号分隔）"
          class="full"
          :value="jvm_str"
          @input="jvm_str = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="游戏参数（空格/逗号分隔）"
          class="full"
          :value="game_str"
          @input="game_str = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="启动命令前缀"
          :value="prefix_str"
          @input="prefix_str = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-text-field
          label="启动命令后缀"
          :value="suffix_str"
          @input="suffix_str = ($event.target as any).value"
        ></mdui-text-field>
      </div>
      <mdui-divider></mdui-divider>
      <div class="card-actions">
        <mdui-button :loading="downloading" @click="create_new_instance()">
          <mdui-icon-rocket--outlined slot="icon"></mdui-icon-rocket--outlined>
          新建实例
        </mdui-button>
        <mdui-button variant="tonal" @click="fetch_version_json()"
          >查看 version.json</mdui-button
        >
      </div>
    </mdui-card>

    <mdui-collapse :value="open_sections">
      <mdui-collapse-item value="global">
        <mdui-card variant="outlined" slot="header" class="collapse-head">
          <mdui-icon-settings--outlined
            class="card-icon"
          ></mdui-icon-settings--outlined>
          <span class="collapse-title">全局设置</span>
        </mdui-card>
        <div class="collapse-body">
          <div class="field-grid">
            <mdui-text-field
              label="镜像地址（留空为官方源）"
              :value="mirror_url"
              @input="mirror_url = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="并发数"
              type="number"
              :value="concurrency_str"
              @input="concurrency_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认分辨率宽"
              type="number"
              :value="g_width_str"
              @input="g_width_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认分辨率高"
              type="number"
              :value="g_height_str"
              @input="g_height_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认 JVM 参数"
              class="full"
              :value="g_jvm_str"
              @input="g_jvm_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认游戏参数"
              class="full"
              :value="g_game_str"
              @input="g_game_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认启动命令前缀"
              :value="g_prefix_str"
              @input="g_prefix_str = ($event.target as any).value"
            ></mdui-text-field>
            <mdui-text-field
              label="默认启动命令后缀"
              :value="g_suffix_str"
              @input="g_suffix_str = ($event.target as any).value"
            ></mdui-text-field>
          </div>
          <div class="card-actions">
            <mdui-button variant="tonal" @click="save_global_config()"
              >保存全局配置</mdui-button
            >
          </div>
        </div>
      </mdui-collapse-item>
    </mdui-collapse>

    <div class="status-line">
      <span class="status-dot"></span>
      <span>{{ status }}</span>
    </div>

    <template v-if="finished">
      <p :class="finished.success ? 'ok' : 'err'">
        结果：{{ finished.success ? "成功" : "失败"
        }}<template v-if="finished.error">：{{ finished.error }}</template>
      </p>
    </template>

    <template v-if="instance_info">
      <mdui-card variant="filled" class="info-card">
        <div class="info-head">
          <div class="card-title">实例信息</div>
          <mdui-chip variant="assist">{{ instance_info.id }}</mdui-chip>
        </div>
        <mdui-list>
          <mdui-list-item>
            <div class="info-row">
              <span>名称</span><b>{{ instance_info.name }}</b>
            </div>
          </mdui-list-item>
          <mdui-list-item>
            <div class="info-row">
              <span>路径</span><b>{{ instance_info.path }}</b>
            </div>
          </mdui-list-item>
          <mdui-list-item>
            <div class="info-row">
              <span>创建时间</span><b>{{ instance_info.createdAt }}</b>
            </div>
          </mdui-list-item>
          <mdui-list-item>
            <div class="info-row">
              <span>分辨率</span
              ><b
                >{{ instance_info.config.width ?? "-" }}×{{
                  instance_info.config.height ?? "-"
                }}</b
              >
            </div>
          </mdui-list-item>
          <mdui-list-item>
            <div class="info-row">
              <span>JVM 参数</span
              ><b class="mono">{{
                instance_info.config.jvmArgs.join(" ") || "-"
              }}</b>
            </div>
          </mdui-list-item>
        </mdui-list>
      </mdui-card>
    </template>

    <mdui-collapse :value="open_sections">
      <mdui-collapse-item value="raw">
        <mdui-card variant="outlined" slot="header" class="collapse-head">
          <mdui-icon-code--outlined
            class="card-icon"
          ></mdui-icon-code--outlined>
          <span class="collapse-title">version.json 原始内容</span>
        </mdui-card>
        <pre>{{ test_text }}</pre>
      </mdui-collapse-item>
    </mdui-collapse>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 820px;
  margin: 0 auto;
  padding: 20px 16px 40px;
}

.page-head h2 {
  margin: 0 0 4px;
}

.page-desc {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.card-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
}

.head-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.arrow-back-button {
  transform: translateY(-2px);
}

.card-icon {
  font-size: 28px;
  color: rgb(var(--mdui-color-primary));
}

.card-title {
  font-weight: 600;
}

.card-sub {
  font-size: 13px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px 16px;
  padding: 16px;
}

.field-grid .full {
  grid-column: 1 / -1;
}

.card-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  padding: 12px 16px;
}

.collapse-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
}

.collapse-title {
  font-weight: 500;
}

.collapse-body {
  padding: 0 16px 8px;
}

.status-line {
  display: flex;
  align-items: center;
  gap: 8px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgb(var(--mdui-color-primary));
  flex-shrink: 0;
}

.ok {
  color: rgb(var(--mdui-color-primary));
}

.err {
  color: rgb(var(--mdui-color-error));
}

.info-card {
  padding: 8px 16px;
}

.info-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  width: 100%;
}

.info-row span {
  color: rgb(var(--mdui-color-on-surface-variant));
  flex-shrink: 0;
}

.info-row b {
  text-align: right;
  word-break: break-all;
}

.mono {
  font-family: ui-monospace, monospace;
  font-size: 13px;
}

pre {
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 300px;
  overflow: auto;
  background: rgb(var(--mdui-color-surface-container));
  border: 1px solid rgb(var(--mdui-color-outline));
  border-radius: var(--mdui-shape-corner-medium);
  padding: 12px;
  margin: 0 16px 16px;
}

@media (max-width: 640px) {
  .field-grid {
    grid-template-columns: 1fr;
  }
}
</style>
