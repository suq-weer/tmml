<script setup lang="ts">
import { RouterView } from "vue-router";
import { useRouteTransitionName } from "../libs/route_transition";

const transitionName = useRouteTransitionName();
</script>

<template>
  <div class="route-transition-view">
    <RouterView v-slot="{ Component, route: viewRoute }">
      <Transition :name="transitionName">
        <component :is="Component" :key="viewRoute.path" />
      </Transition>
    </RouterView>
  </div>
</template>

<style scoped>
.route-transition-view {
  position: relative;
  min-width: 0;
  overflow-x: clip;
}
</style>
