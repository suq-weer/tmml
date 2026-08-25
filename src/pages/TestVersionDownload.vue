<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { VersionContent } from '../libs/mc_version';
import { ref } from 'vue';

var test_text = ref("");

function download() {
    invoke<VersionContent>(
        "get_version",
        {}
    ).then((v) => {
        console.log(v)
        test_text.value = JSON.stringify(v)
    })
}
</script>

<template>
    <h2>版本下载测试页</h2>
    <mdui-button @click="download">下载 1.21.1 的 version.json</mdui-button>
    <mdui-button>检查需要下载的资源</mdui-button>
    <p>{{ test_text }}</p>
</template>