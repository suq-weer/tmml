<script lang="ts" setup>
import { ref } from 'vue';
import { list_instances, MinecraftInstance } from '../libs/instance';
import { get_instance_icon } from '../libs/profile';
import MCInstListOnce from '../components/mc_versions/MCInstListOnce.vue';
import { launch_backend, demo_session, RunningInstance } from '../libs/running';

const DEMO_INSTANCES: MinecraftInstance[] = [
    {
        id: 'demo-1.21.1',
        versionId: '1.21.1',
        name: '生存测试服',
        path: 'demo/1.21.1',
        createdAt: '2026-01-01T00:00:00Z',
    },
    {
        id: 'demo-1.20.4',
        versionId: '1.20.4',
        name: '整合包-原版增强',
        path: 'demo/1.20.4',
        createdAt: '2026-01-02T00:00:00Z',
    },
    {
        id: 'demo-1.21.4',
        versionId: '1.21.4',
        name: '创造红石实验',
        path: 'demo/1.21.4',
        createdAt: '2026-01-03T00:00:00Z',
    },
];

const instance_list = ref<MinecraftInstance[]>();
const backend_available = typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';

list_instances()
    .then((v) => {
        instance_list.value = v.length > 0 ? v : DEMO_INSTANCES;
    })
    .catch(() => {
        // 后端不可用（纯前端预览）时提供一组演示实例用于测试启动流程
        instance_list.value = DEMO_INSTANCES;
    });

async function icon_of(dir: string): Promise<string | null> {
    if (!backend_available) return null;
    try {
        return await get_instance_icon(dir);
    } catch {
        return null;
    }
}

async function on_launch(instance: MinecraftInstance) {
    const dir = instance.path.split('/').pop() ?? instance.path;
    const icon = await icon_of(dir);
    const run: RunningInstance = {
        id: instance.id,
        name: instance.name,
        versionId: instance.versionId,
        path: instance.path,
        icon,
    };
    // mod test
    if (!backend_available) {
        demo_session(run);
        return;
    }
    try {
        await launch_backend(run);
    } catch (e) {
        // 后端可达但启动被拒绝（实例/档案未就绪等），仅打印并保留界面供查看
        console.error('启动失败:', e);
    }
}
</script>

<template class="page">
    <mdui-list>
        <template v-if="instance_list">
            <MCInstListOnce v-for="once in instance_list" v-bind="once" @launch="on_launch" />
        </template>
    </mdui-list>
</template>

<style scoped>
.page {
    min-height: 100vh;
}
</style>
