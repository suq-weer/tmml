<script setup lang="ts">
import "@mdui/icons/task-alt.js";
import { computed, ref } from "vue";
import { useRoute } from "vue-router";
import StepActions from "./StepActions.vue";
import { create_instance } from "../../libs/instance";
import { single_version_from_query } from "../../libs/install_wizard";
import {
  build_instance_config,
  get_customize_values,
  instance_display_name,
  type InstanceCustomizeValues,
} from "../../libs/instance_customize";
import { SUPPORTED_LOADERS } from "../../libs/loader";
import { useToastStore } from "../../libs/toast";

const route = useRoute();
const { pushToast } = useToastStore();

const LOADER_KEY = "loader";

const version = computed(() => single_version_from_query(route.query));
const version_id = computed(() => String(version.value?.id ?? ""));

const loader = computed(() => {
  const value = route.query[LOADER_KEY];
  const raw = typeof value === "string" ? value : Array.isArray(value) ? value[0] : "";
  return SUPPORTED_LOADERS.find((l) => l.id === raw) ?? null;
});

const customize = ref<InstanceCustomizeValues>(
  get_customize_values(version_id.value),
);

const instance_name = computed(() =>
  version.value ? instance_display_name(customize.value, version_id.value) : "",
);

function arg_count(values: InstanceCustomizeValues, key: "jvmArgs" | "gameArgs" | "launchCommandPrefix" | "launchCommandSuffix"): number {
  return values[key].length;
}

/** 直接发起下载任务并放行提交页，不等待下载过程（进度由后端 toast 事件驱动） */
function startTask(): boolean {
  if (!version.value) return false;
  create_instance(
    version_id.value,
    instance_display_name(customize.value, version_id.value),
    build_instance_config(customize.value),
  ).catch((e) => {
    pushToast({
      level: "error",
      title: "提交下载任务失败",
      message: String(e),
    });
  });
  return true;
}
</script>

<template>
  <div class="step-page">
    <section v-if="!version" class="empty-state">
      <p class="empty-title">尚未选择要安装的 Minecraft 版本</p>
      <sub class="empty-desc">
        请先在下载页选择一个游戏版本，再回到本向导继续
      </sub>
    </section>

    <template v-else>
      <mdui-card variant="outlined" class="summary-card">
        <div class="card-head">
          <mdui-icon-task-alt class="card-icon"></mdui-icon-task-alt>
          <div>
            <div class="card-title">任务预览</div>
            <div class="card-sub">
              提交后即开始下载游戏核心并创建实例，配置将在完成后写入本地
            </div>
          </div>
        </div>
        <mdui-divider></mdui-divider>

        <div class="rows">
          <div class="row">
            <span>实例名称</span>
            <b>{{ instance_name }}</b>
          </div>
          <div class="row">
            <span>游戏版本</span>
            <b>{{ version.id }}</b>
          </div>
          <div class="row">
            <span>模组加载器</span>
            <b v-if="loader">{{ loader.name }}</b>
            <b v-else class="dim">不安装（纯净版）</b>
          </div>
          <div class="row">
            <span>JVM 参数</span>
            <b v-if="arg_count(customize, 'jvmArgs') > 0">
              {{ arg_count(customize, "jvmArgs") }} 个
            </b>
            <b v-else class="dim">使用全局默认</b>
          </div>
          <div class="row">
            <span>启动命令</span>
            <b v-if="arg_count(customize, 'launchCommandPrefix') + arg_count(customize, 'launchCommandSuffix') > 0">
              前缀/后缀各
              {{ arg_count(customize, "launchCommandPrefix") }} /
              {{ arg_count(customize, "launchCommandSuffix") }} 个
            </b>
            <b v-else class="dim">无</b>
          </div>
          <div class="row">
            <span>游戏参数</span>
            <b v-if="arg_count(customize, 'gameArgs') > 0">
              {{ arg_count(customize, "gameArgs") }} 个
            </b>
            <b v-else class="dim">使用全局默认</b>
          </div>
          <div class="row">
            <span>窗口分辨率</span>
            <b v-if="customize.width || customize.height">
              {{ customize.width ?? "默认" }} × {{ customize.height ?? "默认" }}
            </b>
            <b v-else class="dim">使用全局默认</b>
          </div>
        </div>

        <sub class="footnote">
          下载进度与详细任务状态可在通知中心实时查看，并支持中途取消
        </sub>
      </mdui-card>
    </template>

    <StepActions
      :allow-next="!!version"
      :before-next="startTask"
    />
  </div>
</template>

<style scoped>
.step-page {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem;
  border: 1px dashed rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
  text-align: center;
}

.empty-title {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.empty-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
  opacity: 0.85;
}

.summary-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
}

.card-icon {
  font-size: 1.75rem;
  color: rgb(var(--mdui-color-primary));
}

.card-title {
  font-size: var(--mdui-typescale-title-medium-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
}

.card-sub {
  font-size: 0.8125rem;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.rows {
  display: flex;
  flex-direction: column;
  padding: 0.5rem 1.25rem 0.25rem;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.45rem 0;
  border-bottom: 1px solid rgb(var(--mdui-color-outline-variant));
}

.row span {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.row b {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.row b.dim {
  color: rgb(var(--mdui-color-on-surface-variant));
  font-weight: normal;
}

.footnote {
  padding: 0.75rem 1.25rem 1rem;
  color: rgb(var(--mdui-color-on-surface-variant));
}
</style>
