<script setup lang="ts">
import { RouterView, useRouter } from 'vue-router';

const router = useRouter();
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
</style>
