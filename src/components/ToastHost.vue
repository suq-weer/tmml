<script lang="ts" setup>
import '@mdui/icons/info--outlined.js';
import '@mdui/icons/check-circle--outlined.js';
import '@mdui/icons/warning--outlined.js';
import '@mdui/icons/error--outlined.js';
import '@mdui/icons/close.js';
import { listen } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRunningStore } from '../libs/running';
import {
    DownloadProgressPayload, ToastItem, ToastLevel, ToastPayload, useToastStore,
} from '../libs/toast';

const { toasts, pushToast, dismissToast, handleDownloadProgress } = useToastStore();
const { dockClearance } = useRunningStore();

const TOAST_CARD_HEIGHT = 96;
const MARGIN = 16;
const viewport_height = ref(window.innerHeight);

/** 底部偏移：为底部胶囊 Dock 让出空间 */
const toast_bottom = computed(() =>
    dockClearance.value > 0 ? Math.max(dockClearance.value, 16) : 16
);

function on_resize() {
    viewport_height.value = window.innerHeight;
}

const max_visible = computed(() =>
    Math.max(1, Math.floor((viewport_height.value - MARGIN * 2) / TOAST_CARD_HEIGHT))
);

const visible_toasts = computed(() => toasts.value.slice(-max_visible.value));

let unlisten_toast: (() => void) | null = null;
let unlisten_progress: (() => void) | null = null;

onMounted(async () => {
    window.addEventListener('resize', on_resize);
    unlisten_toast = await listen<ToastPayload>('toast', (e) => {
        pushToast(e.payload);
    });
    unlisten_progress = await listen<DownloadProgressPayload>('minecraft-download-progress', (e) => {
        handleDownloadProgress(e.payload);
    });
});

onUnmounted(() => {
    window.removeEventListener('resize', on_resize);
    unlisten_toast?.();
    unlisten_progress?.();
});

function level_icon(level: ToastLevel): string {
    switch (level) {
        case 'success': return 'check-circle--outlined';
        case 'warning': return 'warning--outlined';
        case 'error': return 'error--outlined';
        default: return 'info--outlined';
    }
}

function level_color(level: ToastLevel): string {
    switch (level) {
        case 'warning': return 'rgb(var(--mdui-color-tertiary))';
        case 'error': return 'rgb(var(--mdui-color-error))';
        default: return 'rgb(var(--mdui-color-primary))';
    }
}

function fmt_bytes(n: number): string {
    if (n >= 1024 ** 3) return (n / 1024 ** 3).toFixed(2) + " GB";
    if (n >= 1024 ** 2) return (n / 1024 ** 2).toFixed(2) + " MB";
    if (n >= 1024) return (n / 1024).toFixed(1) + " KB";
    return n + " B";
}

function toast_body(t: ToastItem): string {
    if (!t.download) return t.message ?? '';
    if (t.download.finished) return t.download.message ?? '';
    const d = t.download;
    let s = `阶段 ${d.phase} | 文件 ${d.index}/${d.count}`;
    s += ` | ${fmt_bytes(d.bytesDone)}/${fmt_bytes(d.bytesTotal)}`;
    if (d.speed > 0) s += ` | ${fmt_bytes(d.speed)}/s`;
    return s;
}

function download_percent(t: ToastItem): number {
    const d = t.download;
    if (!d || d.bytesTotal === 0) return 0;
    return Math.min(d.bytesDone / d.bytesTotal, 1);
}
</script>

<template>
    <div class="toast-host" :style="{ bottom: toast_bottom + 'px' }">
        <transition-group name="toast" tag="div" class="toast-stack">
            <mdui-card
                v-for="t in visible_toasts"
                :key="t.id"
                :class="['toast-card', t.download && !t.download.finished ? 'toast-card-download' : '']"
            >
                <div class="toast-content">
                    <component
                        :is="'mdui-icon-' + level_icon(t.level)"
                        class="toast-icon"
                        :style="{ color: level_color(t.level) }"
                    ></component>
                    <div class="toast-body">
                        <p class="toast-title">{{ t.title }}</p>
                        <p class="toast-message">{{ toast_body(t) }}</p>
                    </div>
                    <mdui-button-icon class="toast-close" @click="dismissToast(t.id)">
                        <mdui-icon-close></mdui-icon-close>
                    </mdui-button-icon>
                </div>
                <mdui-linear-progress
                    v-if="t.download && !t.download.finished"
                    class="toast-bar"
                    :value="download_percent(t)"
                ></mdui-linear-progress>
            </mdui-card>
        </transition-group>
    </div>
</template>

<style scoped>
.toast-host {
    position: fixed;
    right: 16px;
    z-index: 2000;
    pointer-events: none;
    transition: bottom 0.3s var(--mdui-motion-easing-standard);
}

.toast-stack {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
}

.toast-card {
    width: min(360px, calc(100vw - 32px));
    overflow: hidden;
    pointer-events: auto;
}

.toast-content {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 8px 12px 16px;
}

.toast-card-download .toast-content {
    padding-bottom: 8px;
}

.toast-icon {
    font-size: 24px;
    flex-shrink: 0;
    margin-top: 2px;
}

.toast-body {
    flex: 1;
    min-width: 0;
}

.toast-title {
    font-weight: 600;
    margin: 0;
}

.toast-message {
    margin: 2px 0 0 0;
    color: rgb(var(--mdui-color-on-surface-variant));
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

.toast-close {
    flex-shrink: 0;
    margin-top: -4px;
}

.toast-bar {
    display: block;
    width: 100%;
    border-radius: 0;
    --shape-corner: 0px;
}

.toast-enter-active,
.toast-leave-active {
    transition: all 0.25s ease;
}

.toast-enter-from {
    opacity: 0;
    transform: translateY(12px);
}

.toast-leave-to {
    opacity: 0;
    transform: translateY(-8px);
}
</style>
