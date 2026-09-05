<script setup lang="ts">
import { useRoute } from "vue-router";
import Stepper from "../components/Stepper.vue";
import { SingleVersion } from "../libs/mc_version.ts";
import { leaveCurrentSection } from "../libs/navigation.ts";
import { ref } from "vue";

const route = useRoute();
const version = history.state?.version as SingleVersion;

const step = ref(0);
const steps = ref(["环境配置", "个性化", "开始下载"]);

function goBack(): void {
  leaveCurrentSection("/install");
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div class="head-row">
        <mdui-button-icon class="arrow-back-button" @click="goBack">
          <mdui-icon-arrow-back></mdui-icon-arrow-back>
        </mdui-button-icon>
        <h2>实例创建向导</h2>
      </div>
      <p class="page-desc">一步步带您创建一个新的 Minecraft 实例</p>
    </header>
    <div style="display: block">
      <mdui-card class="stepper float-hover-card" variant="outlined">
        <Stepper class="content" :steps="steps" :step="step" />
      </mdui-card>
      <div class="guide-view">
        <RouterView />
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px clamp(16px, 4vw, 32px) 48px;
}

.page-head h2 {
  margin: 0 0 4px;
}

.page-desc {
  margin: 0;
  margin-left: 3rem !important;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.head-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.arrow-back-button {
  transform: translateY(-2px);
}

.stepper {
  position: sticky;
  width: 100%;
  top: 1rem;
  background-color: rgba(var(--mdui-color-surface), 0.8) !important;
  backdrop-filter: blur(2px);
}
.stepper .content {
  border-radius: var(--mdui-shape-corner-medium);
}
.guide-view {
  margin: 0 1rem;
}
</style>
