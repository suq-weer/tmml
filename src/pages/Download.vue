<script lang="ts" setup>
import { RouterView, useRoute, useRouter } from "vue-router";
import { leaveCurrentSection } from "../libs/navigation";
import "@mdui/icons/gamepad.js";
import "@mdui/icons/backpack.js";
import "@mdui/icons/pie-chart-outline.js";
import "@mdui/icons/interests.js";
import "@mdui/icons/wb-twilight.js";
import "@mdui/icons/map.js";

const route = useRoute();
const router = useRouter();

const sections = [
  {
    subheader: "实例安装",
    items: [
      { key: "game", label: "游戏", icon: "gamepad" },
      { key: "modpack", label: "整合包", icon: "backpack" },
    ],
  },
  {
    subheader: "附加项",
    items: [
      { key: "mod", label: "模组", icon: "pie-chart-outline" },
      { key: "resourcepack", label: "资源包", icon: "interests" },
      { key: "shader", label: "光影", icon: "wb-twilight" },
      { key: "map", label: "地图", icon: "map" },
    ],
  },
];

function navigate(key: string) {
  router.push(`/download/${key}`);
}

function isActive(key: string): boolean {
  return route.path === `/download/${key}`;
}

function goBack(): void {
  leaveCurrentSection("/download");
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div class="head-row">
        <mdui-button-icon class="arrow-back-button" @click="goBack">
          <mdui-icon-arrow-back></mdui-icon-arrow-back>
        </mdui-button-icon>
        <h2>下载页</h2>
      </div>
      <p class="page-desc">下载 Minecraft 最新版本、整合包、模组等内容。</p>
    </header>

    <div class="dashboard">
      <section class="column column-left">
        <div class="nav-sticky">
          <mdui-list>
            <template v-for="section in sections" :key="section.subheader">
              <mdui-list-subheader>{{ section.subheader }}</mdui-list-subheader>
              <mdui-list-item
                class="dl-list-item"
                v-for="item in section.items"
                :key="item.key"
                clickable
                rounded
                :active="isActive(item.key)"
                @click="navigate(item.key)"
              >
                <component :is="`mdui-icon-${item.icon}`" slot="icon" />
                {{ item.label }}
              </mdui-list-item>
            </template>
          </mdui-list>
        </div>
      </section>

      <aside class="column column-right">
        <RouterView />
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

.dashboard {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
  gap: 16px;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.nav-sticky {
  position: sticky;
  top: 0px;
}

.dl-list-item {
  margin-bottom: 0.2rem;
}

@media (max-width: 920px) {
  .dashboard {
    grid-template-columns: 1fr;
  }

  .nav-sticky {
    position: static;
  }
}
</style>
