<script lang="ts" setup>
import DoYouKnowCard from "../components/cards/DoYouKnowCard.vue";
import InstancesListCard from "../components/cards/InstancesListCard.vue";
import SystemCheckCard from "../components/cards/SystemCheckCard.vue";
</script>

<template>
  <div class="page">
    <header class="page-head">
      <h2>首页</h2>
      <p class="page-desc">The Cake is a lie.</p>
    </header>

    <div class="dashboard">
      <!-- ===================== 左栏：主要内容区 ===================== -->
      <section class="column">
        <InstancesListCard />
      </section>

      <!-- ===================== 右栏：辅助工具区 ===================== -->
      <aside class="column">
        <SystemCheckCard />
        <DoYouKnowCard />
      </aside>
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
  color: rgb(var(--mdui-color-on-surface-variant));
}

.dashboard {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

@media (max-width: 920px) {
  .dashboard {
    grid-template-columns: 1fr;
  }
}
</style>
