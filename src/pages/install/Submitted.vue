<script setup lang="ts">
import "@mdui/icons/notifications.js";
import { useRouter } from "vue-router";

const router = useRouter();

function goNotifications(): void {
  router.push("/notifications");
}
</script>

<template>
  <div class="submitted">
    <mdui-icon-notifications class="submitted-icon"></mdui-icon-notifications>
    <p class="submitted-title">任务已提交，请查看通知中心跟踪详细进度！</p>
    <sub class="submitted-desc">
      下载进度与详细任务状态将实时展示在通知中心
    </sub>
    <mdui-button variant="elevated" @click="goNotifications">
      前往通知中心
    </mdui-button>
  </div>
</template>

<style scoped>
.submitted {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  min-height: 260px;
  padding: 32px 16px;
  border: 1px dashed rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
  color: rgb(var(--mdui-color-on-surface-variant));
  text-align: center;
}

.submitted-icon {
  font-size: 64px;
  color: rgb(var(--mdui-color-primary));
}

.submitted-title {
  margin: 0;
}

.submitted-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
}
</style>
