<script setup lang="ts">
import "@mdui/icons/arrow-back.js";
import "@mdui/icons/arrow-forward.js";
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  INSTALL_STEPS,
  INSTALL_SUBMITTED_PATH,
  install_step_index,
} from "../../libs/install_wizard";

const props = withDefaults(
  defineProps<{
    allowNext?: boolean;
    /** 前进前执行的勾子：返回 false 将阻止跳转（用于表单校验等） */
    beforeNext?: () => boolean | Promise<boolean>;
    /** 提交中：控制前进按钮的 loading/禁用态（如开始下载） */
    nextPending?: boolean;
  }>(),
  { allowNext: true, beforeNext: undefined, nextPending: false },
);

const route = useRoute();
const router = useRouter();

const index = computed(() => install_step_index(route.path));
const finished = computed(() => index.value === INSTALL_STEPS.length - 1);
const prev = computed(() =>
  index.value > 0 ? INSTALL_STEPS[index.value - 1] : null,
);
const next = computed(() =>
  finished.value ? null : INSTALL_STEPS[index.value + 1],
);

function goPrev(): void {
  if (!prev.value) return;
  router.go(-1);
}

async function runGuard(): Promise<boolean> {
  if (!props.beforeNext) return true;
  const ok = await props.beforeNext();
  return ok !== false;
}

async function goNext(): Promise<void> {
  const target = next.value;
  if (!target || props.nextPending) return;
  if (!(await runGuard())) return;
  router.push({ path: target.path, query: route.query });
}

async function goFinish(): Promise<void> {
  if (props.nextPending) return;
  if (!(await runGuard())) return;
  router.push(INSTALL_SUBMITTED_PATH);
}
</script>

<template>
  <footer class="step-actions">
    <mdui-button variant="tonal" :disabled="!prev" @click="goPrev">
      <mdui-icon-arrow-back slot="icon"></mdui-icon-arrow-back>
      上一步{{ prev ? `：${prev.label}` : "" }}
    </mdui-button>
    <mdui-button
      v-if="!finished"
      variant="elevated"
      :disabled="!props.allowNext || props.nextPending"
      :loading="props.nextPending"
      @click="goNext"
    >
      <mdui-icon-arrow-forward slot="icon"></mdui-icon-arrow-forward>
      下一步：{{ next?.label }}
    </mdui-button>
    <mdui-button
      v-else
      variant="elevated"
      :disabled="!props.allowNext || props.nextPending"
      :loading="props.nextPending"
      @click="goFinish"
    >
      <mdui-icon-arrow-forward slot="icon"></mdui-icon-arrow-forward>
      {{ props.nextPending ? "正在提交并下载…" : "开始任务" }}
    </mdui-button>
  </footer>
</template>

<style scoped>
.step-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}
</style>
