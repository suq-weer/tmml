<script setup lang="ts">
import "@mdui/icons/check.js";
import "@mdui/icons/extension.js";
import "@mdui/icons/block.js";
import type { LoaderMeta } from "../../libs/loader";

const props = withDefaults(
  defineProps<{
    loader: LoaderMeta;
    selected: boolean;
    /** 是否对当前所选 Minecraft 版本不可用（如 NeoForge < 1.20.1） */
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{
  (e: "select", id: LoaderMeta["id"]): void;
}>();

function toggle(): void {
  if (props.disabled) return;
  emit("select", props.loader.id);
}
</script>

<template>
  <mdui-card
    variant="outlined"
    class="loader-card float-hover-card"
    :class="{ selected: props.selected, disabled: props.disabled }"
  >
    <div
      class="head"
      :class="{ disabled: props.disabled }"
      @click="toggle"
    >
      <mdui-icon-extension class="head-icon"></mdui-icon-extension>
      <div class="head-text">
        <p class="head-name">{{ props.loader.name }}</p>
        <sub class="head-desc">{{ props.loader.desc }}</sub>
      </div>
      <span
        class="head-mark"
        :class="{ selected: props.selected, disabled: props.disabled }"
      >
        <mdui-icon-check
          v-if="props.selected"
          class="mark-check"
        ></mdui-icon-check>
        <mdui-icon-block v-else-if="props.disabled" class="mark-check"></mdui-icon-block>
      </span>
    </div>

    <mdui-divider></mdui-divider>

    <div class="placeholder">
      <p v-if="props.disabled" class="placeholder-title">
        当前 Minecraft 版本不受 {{ props.loader.name }} 支持
      </p>
      <p v-else-if="props.selected" class="placeholder-title">
        已选择，在下方选择加载器版本
      </p>
      <p v-else class="placeholder-title">点击选择</p>
      <sub v-if="props.disabled" class="placeholder-desc">
        {{ props.loader.name }} 需要 Minecraft 1.20.1 及以上的正式版
      </sub>
      <sub v-else-if="!props.selected" class="placeholder-desc">
        选中后将列出该版本可用的加载器版本
      </sub>
      <sub v-else class="placeholder-desc">
        默认选择最新的稳定版本，可在下方调整
      </sub>
    </div>
  </mdui-card>
</template>

<style scoped>
.loader-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.loader-card.selected {
  border-color: rgb(var(--mdui-color-primary));
  box-shadow: var(--mdui-elevation-level2);
}

.loader-card.disabled {
  opacity: 0.6;
}

.head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  cursor: pointer;
}

.head.disabled {
  cursor: not-allowed;
}

.head-icon {
  font-size: 1.75rem;
  color: rgb(var(--mdui-color-primary));
}

.head-text {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
  flex: 1;
  line-height: 1.2;
}

.head-name {
  margin: 0;
  font-size: var(--mdui-typescale-title-medium-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
}

.head-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.head-mark {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: var(--mdui-shape-corner-full);
  border: 2px solid rgb(var(--mdui-color-outline-variant));
  color: rgb(var(--mdui-color-on-surface-variant));
  transition:
    border-color var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard),
    background-color var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard),
    color var(--mdui-motion-duration-short3) var(--mdui-motion-easing-standard);
}

.head-mark.selected {
  border-color: rgb(var(--mdui-color-primary));
  background-color: rgb(var(--mdui-color-primary));
  color: rgb(var(--mdui-color-on-primary));
}

.head-mark.disabled {
  border-color: rgb(var(--mdui-color-outline-variant));
  color: rgb(var(--mdui-color-on-surface-variant));
}

.mark-check {
  font-size: 1rem;
}

.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.35rem;
  padding: 1rem 1rem 1.15rem;
  text-align: center;
}

.placeholder-title {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.placeholder-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
  opacity: 0.8;
}
</style>
