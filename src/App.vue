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
            <mdui-avatar src="/src/assets/mc_icon.png"></mdui-avatar>
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
