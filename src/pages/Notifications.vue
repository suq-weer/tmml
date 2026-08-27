<script lang="ts" setup>
import '@mdui/icons/info--outlined.js';
import '@mdui/icons/check-circle--outlined.js';
import '@mdui/icons/warning--outlined.js';
import '@mdui/icons/error--outlined.js';
import '@mdui/icons/close.js';
import DownloadToastDetail from '../components/DownloadToastDetail.vue';
import { ToastLevel, useToastStore } from '../libs/toast';

const { notifications, removeNotification, markAllRead } = useToastStore();

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

function fmt_time(ts: number): string {
    const d = new Date(ts);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
</script>

<template>
    <div class="notifications-page">
        <div class="head">
            <h2>通知</h2>
            <div class="actions">
                <mdui-button variant="tonal" @click="markAllRead()">全部标记已读</mdui-button>
            </div>
        </div>

        <mdui-list v-if="notifications.length > 0" class="list">
            <mdui-list-item v-for="n in notifications" :key="n.id" class="item">
                <component
                    :is="'mdui-icon-' + level_icon(n.level)"
                    slot="icon"
                    class="item-icon"
                    :style="{ color: level_color(n.level) }"
                ></component>
                <div class="item-text">
                    <p class="item-title">{{ n.title }}</p>
                    <download-toast-detail v-if="n.download" :state="n.download" />
                    <p v-else-if="n.message" class="item-message">{{ n.message }}</p>
                    <sub class="item-time">{{ fmt_time(n.timestamp) }}</sub>
                </div>
                <mdui-button-icon slot="end-icon" class="item-close" @click="removeNotification(n.id)">
                    <mdui-icon-close></mdui-icon-close>
                </mdui-button-icon>
            </mdui-list-item>
        </mdui-list>

        <div v-else class="empty">
            <p>暂无未读通知</p>
        </div>
    </div>
</template>

<style scoped>
.notifications-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
}

.head {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.list {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.item-text {
    display: block;
    line-height: 1.4;
}

.item-title {
    font-weight: 600;
    margin: 0;
}

.item-message {
    margin: 2px 0 0 0;
    color: rgb(var(--mdui-color-on-surface-variant));
    white-space: pre-wrap;
    word-break: break-all;
}

.item-time {
    color: rgb(var(--mdui-color-on-surface-variant));
}

.item-icon {
    font-size: 22px;
}

.item-close {
    display: none;
}

.item:hover .item-close {
    display: block;
}

.empty {
    text-align: center;
    color: rgb(var(--mdui-color-on-surface-variant));
    padding: 48px 0;
}
</style>
