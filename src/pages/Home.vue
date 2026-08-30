<script setup lang="ts">
import MCVerListOnce from '../components/mc_versions/MCVerListOnce.vue';
import { VersionManifest } from '../libs/mc_version.ts';
import { ref } from 'vue';
import { fetch_mc_version_paged } from '../libs/query_backend.ts';

var mc_ver = ref<VersionManifest | null>();

function fetch_mc_version() {
    fetch_mc_version_paged(20, 1, 'RELEASE').then((v) => {
        mc_ver.value = v
    })
}

</script>

<template>
    <mdui-button @click="fetch_mc_version()">获取 Minecraft 所有版本</mdui-button>
    <mdui-list>
        <template v-if="mc_ver && mc_ver.versions">
            <MCVerListOnce v-for="once in mc_ver.versions" v-bind="once" />
        </template>
    </mdui-list>
</template>
