<script setup lang="ts">
import "@mdui/icons/check.js";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import MCVerListOnce from "../../components/mc_versions/MCVerListOnce.vue";
import LoaderListCard from "../../components/cards/LoaderListCard.vue";
import StepActions from "./StepActions.vue";
import { single_version_from_query } from "../../libs/install_wizard";
import {
  SUPPORTED_LOADERS,
  list_loader_versions,
  type LoaderId,
  type LoaderMeta,
  type LoaderVersion,
} from "../../libs/loader";
import {
  get_loader_choice,
  set_loader_choice,
} from "../../libs/loader_choice";
import { neo_forge_supported } from "../../libs/mc_version";
import { useToastStore } from "../../libs/toast";

const route = useRoute();
const router = useRouter();
const { pushToast } = useToastStore();

const LOADER_KEY = "loader";

const version = computed(() => single_version_from_query(route.query));
const version_id = computed(() => String(version.value?.id ?? ""));
const version_type = computed(() => String(version.value?.type ?? ""));

const selectedKind = computed<LoaderId | null>(() => {
  const v = route.query[LOADER_KEY];
  const value = typeof v === "string" ? v : Array.isArray(v) ? v[0] : null;
  if (typeof value !== "string") return null;
  return SUPPORTED_LOADERS.some((l) => l.id === value)
    ? (value as LoaderId)
    : null;
});

/** 生效的加载器：NeoForge 不受当前版本支持时视作未选择 */
const loader = computed<LoaderMeta | null>(() => {
  const kind = selectedKind.value;
  if (!kind || !version_id.value) return null;
  const meta = SUPPORTED_LOADERS.find((l) => l.id === kind) ?? null;
  if (!meta) return null;
  if (
    kind === "neoforge" &&
    !neo_forge_supported({ id: version_id.value, type: version_type.value })
  ) {
    return null;
  }
  return meta;
});

const loaderName = computed(() => loader.value?.name ?? "");

function is_loader_disabled(kind: LoaderId): boolean {
  if (kind !== "neoforge") return false;
  return !neo_forge_supported({
    id: version_id.value,
    type: version_type.value,
  });
}

// | 加载器版本选择 |

const loaderVersions = ref<LoaderVersion[]>([]);
const loadingVersions = ref(false);
const versionError = ref("");
const pickVersion = ref("");
const withFabricApi = ref(false);
const loadSerial = ref(0);

function commitChoice(): void {
  const mc = version_id.value;
  const kind = loader.value?.id;
  if (!kind || !mc) {
    set_loader_choice(mc, null);
    return;
  }
  set_loader_choice(mc, {
    kind,
    version: pickVersion.value,
    withFabricApi: withFabricApi.value,
  });
}

async function loadLoaderVersions(): Promise<void> {
  const mc = version_id.value;
  const kind = loader.value?.id;
  if (!mc || !kind) {
    loaderVersions.value = [];
    pickVersion.value = "";
    loadingVersions.value = false;
    versionError.value = "";
    set_loader_choice(mc, null);
    return;
  }

  const serial = ++loadSerial.value;
  loadingVersions.value = true;
  versionError.value = "";
  loaderVersions.value = [];
  pickVersion.value = "";
  try {
    const list = await list_loader_versions(kind, mc);
    if (serial !== loadSerial.value) return;
    loaderVersions.value = list;
    if (list.length === 0) {
      pickVersion.value = "";
    } else {
      // 优先沿用之前选过且仍存在的版本；否则取最新 stable
      const cached = get_loader_choice(mc);
      const previous =
        cached && cached.kind === kind && list.some((v) => v.version === cached.version)
          ? cached.version
          : "";
      pickVersion.value =
        previous || (list.find((v) => v.stable) ?? list[0]).version;
      withFabricApi.value = cached?.kind === kind ? cached.withFabricApi : false;
    }
    commitChoice();
  } catch (e) {
    if (serial !== loadSerial.value) return;
    loaderVersions.value = [];
    versionError.value = String(e);
    commitChoice();
  } finally {
    if (serial === loadSerial.value) {
      loadingVersions.value = false;
    }
  }
}

watch(
  () => `${version_id.value}|${selectedKind.value ?? ""}|${version_type.value}`,
  () => {
    loadLoaderVersions();
  },
);
onMounted(() => {
  loadLoaderVersions();
});

// | 交互 |

function toggleLoader(id: LoaderId): void {
  const disabled = is_loader_disabled(id);
  // 当前版本不支持时仍允许「取消」掉之前残留的选择
  const staleSelected = selectedKind.value === id && loader.value === null && disabled;
  if (disabled && !staleSelected) {
    pushToast({
      level: "warning",
      title: "当前版本不支持该加载器",
      message: `${SUPPORTED_LOADERS.find((l) => l.id === id)?.name} 需要 Minecraft 1.20.1 及以上的正式版`,
    });
    return;
  }
  const query = { ...route.query };
  if (loader.value?.id === id || selectedKind.value === id) {
    delete query[LOADER_KEY];
    set_loader_choice(version_id.value, null);
  } else {
    query[LOADER_KEY] = id;
  }
  router.push({ path: "/install/env", query });
}

function onPickVersion(value: string): void {
  pickVersion.value = value;
  commitChoice();
}

function onFabricApiToggle(e: Event): void {
  withFabricApi.value = Boolean((e.target as HTMLInputElement).checked);
  commitChoice();
}

// 前进条件：选了加载器则必须等版本列表就绪并选中版本
const allowNext = computed(() => {
  if (!version.value) return false;
  if (!loader.value) return true;
  return (
    !loadingVersions.value &&
    versionError.value === "" &&
    loaderVersions.value.length > 0 &&
    pickVersion.value !== ""
  );
});

const selectedVersionMeta = computed(() =>
  loaderVersions.value.find((v) => v.version === pickVersion.value),
);
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
          :selected="loader?.id === item.id"
          :disabled="is_loader_disabled(item.id)"
          @select="toggleLoader"
        />
      </div>
    </section>

    <section v-if="loader" class="chunk">
      <mdui-card variant="outlined" class="picker-card float-hover-card">
        <div class="picker-head">
          <div>
            <div class="picker-title">加载器版本</div>
            <sub class="picker-sub">
              选择用于 Minecraft {{ version_id }} 的 {{ loaderName }} 构建
            </sub>
          </div>
        </div>
        <mdui-divider></mdui-divider>

        <div v-if="loadingVersions" class="state-area">
          <mdui-circular-progress></mdui-circular-progress>
          <p class="state-text">正在获取 {{ loaderName }} 版本列表…</p>
        </div>

        <div v-else-if="versionError" class="state-area">
          <p class="state-text error">加载版本列表失败</p>
          <sub class="state-text">{{ versionError }}</sub>
        </div>

        <div v-else-if="loaderVersions.length === 0" class="state-area">
          <p class="state-text">
            {{ loaderName }} 暂无适配 Minecraft {{ version_id }} 的版本
          </p>
        </div>

        <div v-else class="picker-body">
          <div class="chosen-line">
            <span>已选</span>
            <b>{{ pickVersion }}</b>
            <sub v-if="selectedVersionMeta">
              {{ selectedVersionMeta.stable ? "稳定版" : "预览版" }}
            </sub>
          </div>

          <div class="version-list">
            <button
              v-for="v in loaderVersions"
              :key="v.version"
              type="button"
              class="version-item"
              :class="{ active: v.version === pickVersion }"
              @click="onPickVersion(v.version)"
            >
              <span class="version-id">{{ v.version }}</span>
              <span
                class="version-tag"
                :class="v.stable ? 'stable' : 'beta'"
                >{{ v.stable ? "稳定" : "预览" }}</span
              >
              <mdui-icon-check
                v-if="v.version === pickVersion"
                class="mark"
              ></mdui-icon-check>
            </button>
          </div>

          <div v-if="loader.id === 'fabric'" class="api-row">
            <mdui-switch
              :checked="withFabricApi"
              @change="onFabricApiToggle"
            ></mdui-switch>
            <div class="api-text">
              <p class="api-title">同时安装 Fabric API</p>
              <sub class="api-desc">
                许多 Fabric 模组的前置；将作为普通模组放入实例的 mods 目录
              </sub>
            </div>
          </div>
        </div>
      </mdui-card>
    </section>

    <StepActions :allow-next="allowNext" />
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

.picker-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.picker-head {
  padding: 1rem 1.25rem;
}

.picker-title {
  font-size: var(--mdui-typescale-title-medium-size);
  font-weight: var(--mdui-typescale-title-medium-weight);
}

.picker-sub {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.picker-body {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem 1.25rem 1.25rem;
}

.chosen-line {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  flex-wrap: wrap;
  font-size: 0.875rem;
}

.chosen-line span {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.chosen-line sub {
  color: rgb(var(--mdui-color-primary));
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: min(40vh, 320px);
  overflow-y: auto;
  border: 1px solid rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
  padding: 4px;
}

.version-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: none;
  border-radius: var(--mdui-shape-corner-large);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--mdui-motion-duration-short3)
    var(--mdui-motion-easing-standard);
}

.version-item:hover {
  background-color: rgb(var(--mdui-color-surface-container-highest));
}

.version-item.active {
  background-color: color-mix(
    in srgb,
    rgb(var(--mdui-color-primary)) 14%,
    transparent
  );
}

.version-id {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

.version-tag {
  flex-shrink: 0;
  font-size: 0.6875rem;
  line-height: 1;
  padding: 0.25rem 0.45rem;
  border-radius: var(--mdui-shape-corner-full);
}

.version-tag.stable {
  color: rgb(var(--mdui-color-on-primary));
  background-color: rgb(var(--mdui-color-primary));
}

.version-tag.beta {
  color: rgb(var(--mdui-color-on-secondary-container));
  background-color: rgb(var(--mdui-color-secondary-container));
}

.version-item .mark {
  flex-shrink: 0;
  font-size: 1.1rem;
  color: rgb(var(--mdui-color-primary));
}

.api-row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
}

.api-text {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
}

.api-title {
  margin: 0;
}

.api-desc {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.state-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.state-text {
  margin: 0;
}

.state-text.error {
  color: rgb(var(--mdui-color-error));
}
</style>
