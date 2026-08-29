<script setup lang="ts">
import { RouterView, useRouter } from 'vue-router';
import { computed } from 'vue';
import ToastHost from './components/ToastHost.vue';
import { useToastStore } from './libs/toast';
import '@mdui/icons/notifications';

const router = useRouter();
const { notifications } = useToastStore();
const unread_count = computed(() => notifications.value.length);

function push(route: string) {
    router.push(route)
}
</script>

<template>
    <mdui-layout class="div">
        <mdui-top-app-bar variant="large" class="title" scroll-behavior="shrink elevate" scroll-target=".main-page">
            <img class="mc-avatar" src="/src/assets/mc_icon.png" alt="avatar" />
            <mdui-top-app-bar-title>
                Too Many Minecraft Launcher
                <span slot="label-large">欢迎！XiaosuLikeJvav</span>
            </mdui-top-app-bar-title>
            <div style="flex-grow: 1"></div>
            <div class="notif-wrap">
                <mdui-button-icon @click="push('/notifications')">
                    <mdui-icon-notifications></mdui-icon-notifications>
                </mdui-button-icon>
                <mdui-badge v-if="unread_count > 0" class="notif-badge">{{ unread_count }}</mdui-badge>
            </div>
        </mdui-top-app-bar>
        <!--<mdui-layout-item placement="left">
            <side-bar />
        </mdui-layout-item>-->
        <mdui-layout-main class="main-page">
            <mdui-button @click="push('/')">/</mdui-button>
            <mdui-button @click="push('/version')">/version</mdui-button>
            <router-view />
        </mdui-layout-main>
    </mdui-layout>
    <toast-host />
</template>

<style lang="css">
.div {
    height: 100%;
    position: relative;
    overflow: hidden;
}
.main-page {
    height: 100%;
    overflow: auto;
}
mdui-top-app-bar[variant="large"] {
    height: 12rem !important;
}
mdui-top-app-bar[variant="large"][shrink]:not([shrink="false" i]) {
    height: 4rem !important;
}
.mc-avatar {
    width: 6rem;
    height: 6rem;
    margin-left: 1rem !important;
    aspect-ratio: 1 / 1;
    object-fit: cover;
    image-rendering: pixelated;
    border-radius: var(--mdui-shape-corner-medium);
    margin-top: 1rem;
    flex-shrink: 0;
    transition:
        width var(--mdui-motion-duration-short4) var(--mdui-motion-easing-standard),
        height var(--mdui-motion-duration-short4) var(--mdui-motion-easing-standard);
}
mdui-top-app-bar[shrink]:not([shrink="false" i]) .mc-avatar {
    width: 1.875rem;
    height: 1.875rem;
    margin-left: 0 !important;
    margin-top: 5px !important;
}
.notif-wrap {
    position: relative;
    display: inline-flex;
}
.notif-badge {
    position: absolute;
    top: -2px;
    right: -4px;
}
</style>
