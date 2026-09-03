<script lang="ts" setup>
import { ref } from "vue";
import { list_instances, MinecraftInstance } from "../libs/instance.ts";
import { get_instance_icon } from "../libs/profile.ts";
import MCInstListOnce from "./mc_versions/MCInstListOnce.vue";
import { launch_backend, RunningInstance } from "../libs/running.ts";

const instance_list = ref<MinecraftInstance[]>();
const backend_available =
  typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

list_instances().then((v) => {
  instance_list.value = v;
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
  const dir = instance.path.split("/").pop() ?? instance.path;
  const icon = await icon_of(dir);
  const run: RunningInstance = {
    id: instance.id,
    name: instance.name,
    versionId: instance.versionId,
    path: instance.path,
    icon,
  };
  try {
    await launch_backend(run);
  } catch (e) {
    console.error("启动失败:", e);
  }
}
</script>

<template class="page">
  <mdui-list>
    <template v-if="instance_list">
      <MCInstListOnce
        v-for="once in instance_list"
        v-bind="once"
        @launch="on_launch"
      />
    </template>
    <span class="none-instance" v-else
      >启动器目录中未找到实例，快去新建一个吧~</span
    >
  </mdui-list>
</template>

<style scoped>
.page {
  min-height: 100vh;
}

.none-instance {
  margin: 1rem;
}
</style>
