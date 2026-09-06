<script setup lang="ts">
import { useRoute } from "vue-router";
import Stepper from "../components/Stepper.vue";
import RouteTransitionView from "../components/RouteTransitionView.vue";
import { leaveCurrentSection } from "../libs/navigation.ts";
import {
  INSTALL_STEPS,
  INSTALL_SUBMITTED_PATH,
  install_step_index,
} from "../libs/install_wizard";
import { computed } from "vue";
import "@mdui/icons/arrow-back.js";

const route = useRoute();

const step = computed(() =>
  route.path === INSTALL_SUBMITTED_PATH
    ? INSTALL_STEPS.length
    : install_step_index(route.path),
);
const steps = computed(() => INSTALL_STEPS.map((s) => s.label));

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
      <Stepper class="content stepper" :steps="steps" :step="step" />
      <div class="guide-view">
        <RouteTransitionView />
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
  padding: 0.5rem;
  width: 100%;
  top: 1rem;
  z-index: 1145;
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
