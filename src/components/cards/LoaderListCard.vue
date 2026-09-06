<script setup lang="ts">
import "@mdui/icons/check.js";
import "@mdui/icons/extension.js";
import type { LoaderMeta } from "../../libs/loader";

const props = defineProps<{
  loader: LoaderMeta;
  selected: boolean;
}>();

const emit = defineEmits<{
  (e: "select", id: LoaderMeta["id"]): void;
}>();

function toggle(): void {
  emit("select", props.loader.id);
}
</script>

<template>
  <mdui-card
    variant="outlined"
    class="loader-card float-hover-card"
    :class="{ selected: props.selected }"
  >
    <div class="head" @click="toggle">
      <mdui-icon-extension class="head-icon"></mdui-icon-extension>
      <div class="head-text">
        <p class="head-name">{{ props.loader.name }}</p>
        <sub class="head-desc">{{ props.loader.desc }}</sub>
      </div>
      <span class="head-mark" :class="{ selected: props.selected }">
        <mdui-icon-check class="mark-check"></mdui-icon-check>
      </span>
    </div>

    <mdui-divider></mdui-divider>

    <div class="placeholder">
      <p class="placeholder-title">
        {{ props.selected ? "已选择，等待接入版本数据" : "版本列表待接入" }}
      </p>
      <sub class="placeholder-desc">
        将根据所选的游戏版本，在这里列出
        {{ props.loader.name }} 可用的加载器版本
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

.head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  cursor: pointer;
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
