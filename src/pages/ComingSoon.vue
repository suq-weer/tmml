<script lang="ts" setup>
import "@mdui/icons/construction.js";
</script>

<template>
  <div class="placeholder">
    <mdui-icon-construction class="placeholder-icon"></mdui-icon-construction>
    <p>该板块正在建设中，敬请期待！</p>
    <sub>模组加载器安装功能都还没开发呢……</sub>
  </div>
</template>

<style scoped>
.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 200px;
  padding: 32px 16px;
  border: 1px dashed rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
  color: rgb(var(--mdui-color-on-surface-variant));
}

.placeholder-icon {
  font-size: 48px;
  color: rgb(var(--mdui-color-outline));
}
</style>
