<script setup lang="ts">
import "@mdui/icons/apps--outlined.js";
import InstancesList from "../InstancesList.vue";
</script>

<template>
  <mdui-card variant="outlined" class="float-hover-card">
    <div class="card-head">
      <mdui-icon-apps--outlined class="card-icon"></mdui-icon-apps--outlined>
      <div>
        <div class="card-title">实例列表</div>
        <div class="card-sub">点击实例卡片即可启动游戏</div>
      </div>
      <div style="flex-grow: 1" />
      <mdui-button @click="$router.push('/download')">前往下载页</mdui-button>
    </div>
    <mdui-divider></mdui-divider>
    <div class="card-body flush">
      <InstancesList />
    </div>
  </mdui-card>
</template>

<style scoped>
.card-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
}

.card-icon {
  font-size: 28px;
  color: rgb(var(--mdui-color-primary));
}

.card-title {
  font-weight: 800;
}

.card-sub {
  font-size: 13px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
}

.card-body.flush {
  padding: 4px;
}
</style>
