<script setup lang="ts">
import MCVerListOnce from "../../components/mc_versions/MCVerListOnce.vue";
import { SingleVersion } from "../../libs/mc_version.ts";
import { onMounted, ref } from "vue";
import { fetch_mc_version_paged } from "../../libs/query_backend.ts";
import { useToastStore } from "../../libs/toast.ts";
import { VersionMode } from "../../libs/mc_version.ts";
import "@mdui/icons/keyboard-arrow-down.js";

const { pushToast } = useToastStore();
const PAGE_SIZE = 20;

const props = defineProps<{
  version_mode: VersionMode;
}>();

const versions = ref<SingleVersion[]>([]);
const page = ref(1);
const total = ref(0);
const loading = ref(false);
const hasMore = ref(true);
const errorMsg = ref("");
const collapseRef = ref<HTMLElement>();
const isExpanded = ref(false);

async function loadPage() {
  if (loading.value || !hasMore.value) return;
  loading.value = true;
  errorMsg.value = "";
  const data = await fetch_mc_version_paged(
    PAGE_SIZE,
    page.value,
    props.version_mode,
  );
  loading.value = false;
  if (!data) {
    errorMsg.value = "加载失败，请稍后重试";
    pushToast({ level: "error", title: errorMsg.value });
    return;
  }
  versions.value.push(...data.versions);
  total.value = data.total;
  hasMore.value = data.hasMore;
  page.value += 1;
}

function output_version_mode(v: VersionMode): String {
  switch (v) {
    case "ALL":
      return "全部版本";
    case "RELEASE":
      return "正式版";
    case "SNAPSHOT":
      return "快照";
    case "FOOL":
      return "愚人节版本";
  }
}

function output_tips_version_mode(v: VersionMode): String {
  switch (v) {
    case "FOOL":
      return "暂不支持 v2.0 愚人节版本及其变种版本（识别成快照这个 Bug 容我再咕咕一下）";
    case "SNAPSHOT":
      return "Q: 在快照列表中，愚人节版本都分别在哪里？";
    default:
      return "恭喜你终于来到了 Minecraft 最久远的过去，那么它的未来是什么呢？";
  }
}

function rotateIcon(e: MouseEvent) {
  const item = e.currentTarget as HTMLElement;
  const icon = item.querySelector("mdui-icon-keyboard-arrow-down");
  icon?.classList.toggle("rotate");
}

function checkExpand() {
  if (props.version_mode == "RELEASE") {
    collapseRef.value?.setAttribute("value", "item");
    isExpanded.value = true;
  }
}

onMounted(() => {
  loadPage();
  checkExpand();
});
</script>

<template>
  <mdui-card variant="outlined" class="float-hover-card">
    <mdui-collapse accordion ref="collapseRef">
      <mdui-collapse-item value="item">
        <mdui-list-item slot="header" @click="rotateIcon"
          >{{ output_version_mode(props.version_mode) }}
          <mdui-icon-keyboard-arrow-down
            slot="end-icon"
            :class="{ rotate: isExpanded }"
          ></mdui-icon-keyboard-arrow-down>
        </mdui-list-item>
        <mdui-list style="margin: 0 1rem 0 1rem" v-if="versions.length > 0">
          <MCVerListOnce
            v-for="once in versions"
            :key="once.id as any"
            v-bind="once"
          />
        </mdui-list>

        <div v-if="versions.length === 0" class="state-area">
          <p v-if="loading" class="state-text">加载中…</p>
          <p v-else class="state-text">
            {{ errorMsg || "当前类型下暂无版本" }}
          </p>
        </div>

        <div v-else class="state-area">
          <p v-if="!hasMore" class="state-text">
            已经到底啦，共 {{ total }} 个版本
          </p>
          <sub v-if="!hasMore" class="state-text">{{
            output_tips_version_mode(props.version_mode)
          }}</sub>
          <p v-else-if="errorMsg" class="state-text error">{{ errorMsg }}</p>
          <mdui-button
            v-if="hasMore"
            variant="outlined"
            :loading="loading"
            @click="loadPage()"
          >
            {{ errorMsg ? "点击重试" : "加载更多" }}
          </mdui-button>
        </div>
      </mdui-collapse-item>
    </mdui-collapse>
  </mdui-card>
</template>

<style scoped>
.state-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px 0;
}

.state-text {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.state-text.error {
  color: rgb(var(--mdui-color-error));
}

mdui-icon-keyboard-arrow-down {
  transition: transform var(--mdui-motion-duration-short4)
    var(--mdui-motion-easing-standard);
}
mdui-icon-keyboard-arrow-down.rotate {
  transform: rotate(180deg);
}
</style>
