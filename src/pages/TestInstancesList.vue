<script lang="ts" setup>
import { ref } from 'vue';
import { list_instances, MinecraftInstance } from '../libs/instance';
import MCInstListOnce from '../components/mc_versions/MCInstListOnce.vue';

const instance_list = ref<MinecraftInstance[]>();

list_instances().then((v) => {
    instance_list.value = v;
})
</script>

<template class="page">
    <mdui-list>
        <template v-if="instance_list">
            <MCInstListOnce v-for="once in instance_list" v-bind="once" />
        </template>
    </mdui-list>
</template>

<style scoped>
.page {
    min-height: 100vh;
}
</style>
