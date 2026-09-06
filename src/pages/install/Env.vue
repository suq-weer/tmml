<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import MCVerListOnce from "../../components/mc_versions/MCVerListOnce.vue";
import LoaderListCard from "../../components/cards/LoaderListCard.vue";
import StepActions from "./StepActions.vue";
import { single_version_from_query } from "../../libs/install_wizard";
import { SUPPORTED_LOADERS, type LoaderId } from "../../libs/loader";

const route = useRoute();
const router = useRouter();

const LOADER_KEY = "loader";

const version = computed(() => single_version_from_query(route.query));

function loader_from_query(): LoaderId | null {
  const v = route.query[LOADER_KEY];
  const value = typeof v === "string" ? v : Array.isArray(v) ? v[0] : null;
  if (typeof value !== "string") return null;
  return SUPPORTED_LOADERS.some((l) => l.id === value)
    ? (value as LoaderId)
    : null;
}

const loader = computed(() => loader_from_query());

const loaderName = computed(
  () => SUPPORTED_LOADERS.find((l) => l.id === loader.value)?.name ?? "",
);

function toggleLoader(id: LoaderId): void {
  const query = { ...route.query };
  if (loader.value === id) delete query[LOADER_KEY];
  else query[LOADER_KEY] = id;
  router.push({ path: "/install/env", query });
}
</script>

<template>
  <div class="env-step">
    <section class="chunk">
      <div class="chunk-head">
        <h3>已选游戏版本</h3>
      </div>

      <mdui-card
        v-if="version"
        variant="outlined"
        class="version-card float-hover-card"
      >
        <mdui-list>
          <MCVerListOnce v-bind="version" :interactive="false" />
        </mdui-list>
      </mdui-card>

      <mdui-card v-else variant="outlined" class="empty-card float-hover-card">
        <p class="empty-title">尚未选择要安装的 Minecraft 版本</p>
        <sub class="empty-desc">
          请先在下载页选择一个游戏版本，再回到本向导继续
        </sub>
      </mdui-card>
    </section>

    <section class="chunk">
      <div class="chunk-head">
        <h3>模组加载器</h3>
        <sub class="chunk-hint">
          {{
            loader
              ? `已选择 ${loaderName}（再次点击可取消）`
              : "未选择 · 点击下方卡片选择；不选将以纯净版安装"
          }}
        </sub>
      </div>

      <div class="loader-grid">
        <LoaderListCard
          v-for="item in SUPPORTED_LOADERS"
          :key="item.id"
          :loader="item"
          :selected="loader === item.id"
          @select="toggleLoader"
        />
      </div>
    </section>

    <StepActions :allow-next="!!version" />
  </div>
</template>

<style scoped>
.env-step {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.chunk {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.chunk-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.chunk-head h3 {
  margin: 0;
  font-size: var(--mdui-typescale-title-medium-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
}

.chunk-hint {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.loader-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
  gap: 0.75rem;
}

.version-card {
  overflow: hidden;
}

.empty-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem;
  text-align: center;
}

.empty-title {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.empty-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
  opacity: 0.85;
}
</style>
