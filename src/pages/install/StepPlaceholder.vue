<script setup lang="ts">
import "@mdui/icons/construction.js";

defineProps<{
  title: string;
  desc?: string;
  planned?: string[];
}>();
</script>

<template>
  <div class="placeholder">
    <mdui-icon-construction class="placeholder-icon"></mdui-icon-construction>
    <p class="placeholder-title">{{ title }}尚未规划完成</p>
    <sub v-if="desc" class="placeholder-desc">{{ desc }}</sub>
    <div v-if="planned && planned.length" class="planned">
      <span
        v-for="item in planned"
        :key="item"
        class="planned-chip"
      >
        {{ item }}
      </span>
    </div>
    <sub class="placeholder-note">先用占位展示顶替，后续步骤再逐步实现</sub>
  </div>
</template>

<style scoped>
.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 220px;
  padding: 32px 16px;
  border: 1px dashed rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
  color: rgb(var(--mdui-color-on-surface-variant));
}

.placeholder-icon {
  font-size: 48px;
  color: rgb(var(--mdui-color-outline));
}

.placeholder-title {
  margin: 0;
}

.placeholder-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
  text-align: center;
}

.planned {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  margin-top: 4px;
}

.planned-chip {
  padding: 4px 12px;
  border-radius: var(--mdui-shape-corner-full);
  border: 1px solid rgb(var(--mdui-color-outline-variant));
  font-size: var(--mdui-typescale-label-large-size);
  color: rgb(var(--mdui-color-on-surface-variant));
}

.placeholder-note {
  opacity: 0.75;
  margin-top: 4px;
}
</style>
