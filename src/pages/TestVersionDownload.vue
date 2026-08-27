<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { VersionContent } from '../libs/mc_version';
import { onMounted, onUnmounted, ref } from 'vue';

interface DownloadFinished {
    versionId: string;
    success: boolean;
    error: string | null;
}

interface MainConfig {
    accentColor: number;
    downloadConcurrency: number;
    mirrorUrl: string | null;
}

var test_text = ref("");
var version_id = ref("1.21.1");
var mirror_url = ref("");
var concurrency_str = ref("16");
var downloading = ref(false);
var status = ref("尚未开始");
var finished = ref<DownloadFinished | null>(null);

var unlisten_finished: UnlistenFn | null = null;

onMounted(async () => {
    invoke<MainConfig>("get_main_config")
        .then((cfg) => {
            mirror_url.value = cfg.mirrorUrl ?? "";
            concurrency_str.value = String(cfg.downloadConcurrency);
            status.value = "已读取配置：并发 " + cfg.downloadConcurrency + (cfg.mirrorUrl ? "，镜像 " + cfg.mirrorUrl : "，官方源");
        })
        .catch((e) => console.error(e));

    unlisten_finished = await listen<DownloadFinished>("minecraft-download-finished", (e) => {
        finished.value = e.payload;
        downloading.value = false;
        status.value = e.payload.success ? "下载完成" : "下载失败";
    });
});

onUnmounted(() => {
    unlisten_finished?.();
});

function on_version_input(e: Event) {
    version_id.value = (e.target as any).value;
}

function on_mirror_input(e: Event) {
    mirror_url.value = (e.target as any).value;
}

function on_concurrency_input(e: Event) {
    concurrency_str.value = (e.target as any).value;
}

function parse_concurrency(): number {
    const n = Math.trunc(Number(concurrency_str.value));
    return Number.isFinite(n) && n >= 1 ? n : 16;
}

function save_config() {
    invoke<MainConfig>("set_main_config", {
        downloadConcurrency: parse_concurrency(),
        mirrorUrl: mirror_url.value.trim(),
    })
        .then((cfg) => {
            status.value = "配置已保存：并发 " + cfg.downloadConcurrency + (cfg.mirrorUrl ? "，镜像 " + cfg.mirrorUrl : "，官方源");
        })
        .catch((e) => {
            status.value = "保存配置失败: " + e;
        });
}

function download_full() {
    downloading.value = true;
    finished.value = null;
    status.value = "开始下载 " + version_id.value + " ...";
    invoke("download_minecraft_version", { versionId: version_id.value })
        .then(() => {
            status.value = "下载任务已提交完成";
        })
        .catch((e) => {
            status.value = "调用失败: " + e;
            downloading.value = false;
        });
}

function fetch_version_json() {
    invoke<VersionContent>("get_version", {})
        .then((v) => {
            console.log(v);
            test_text.value = JSON.stringify(v);
        })
        .catch((e) => {
            test_text.value = "获取失败: " + e;
        });
}
</script>

<template>
    <div class="download-test">
        <h2>版本下载测试页</h2>

        <mdui-card variant="outlined">
            <div class="config-row">
                <mdui-text-field label="版本号" :value="version_id" @input="on_version_input"></mdui-text-field>
                <mdui-text-field label="镜像地址（留空为官方源）" :value="mirror_url" @input="on_mirror_input"></mdui-text-field>
                <mdui-text-field label="并发数" type="number" :value="concurrency_str" @input="on_concurrency_input"></mdui-text-field>
            </div>
            <div class="btn-row">
                <mdui-button :loading="downloading" @click="download_full()">下载完整 Minecraft 版本</mdui-button>
                <mdui-button variant="tonal" @click="save_config()">保存配置</mdui-button>
                <mdui-button variant="tonal" @click="fetch_version_json()">下载 1.21.1 的 version.json</mdui-button>
            </div>
        </mdui-card>

        <p>状态：{{ status }}</p>

        <template v-if="finished">
            <p :class="finished.success ? 'ok' : 'err'">
                结果：{{ finished.success ? "成功" : "失败" }}<template v-if="finished.error">：{{ finished.error }}</template>
            </p>
        </template>

        <h3>version.json 原始内容</h3>
        <pre>{{ test_text }}</pre>
    </div>
</template>

<style scoped>
.download-test {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
}

.config-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
}

.btn-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
}

.ok {
    color: rgb(var(--mdui-color-primary));
}

.err {
    color: rgb(var(--mdui-color-error));
}

pre {
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 300px;
    overflow: auto;
    background: rgb(var(--mdui-color-surface-container));
    border: 1px solid rgb(var(--mdui-color-outline));
    padding: 8px;
}
</style>
