<script lang="ts" setup>
import "@mdui/icons/memory--outlined.js";
import { invoke } from "@tauri-apps/api/core";
import { onMounted, onUnmounted, ref } from "vue";

interface SystemStats {
  cpuUsage: number;
  totalMem: number;
  usedMem: number;
  cpuCount: number;
}

const backend_available =
  typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

const stats = ref<SystemStats | null>(null);
const load_error = ref(false);

let timer: ReturnType<typeof setInterval> | null = null;
// 纯浏览器预览（无后端）时用于演示的数据
let demo_cpu = 15;

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

function fmt_bytes(b: number): string {
  const gb = b / 1024 ** 3;
  if (gb >= 1) return gb.toFixed(1) + " GB";
  return Math.round(b / 1024 ** 2) + " MB";
}

function cpu_pct(): number {
  return clamp(stats.value?.cpuUsage ?? 0, 0, 100);
}

function mem_pct(): number {
  const s = stats.value;
  if (!s || s.totalMem <= 0) return 0;
  return clamp((s.usedMem / s.totalMem) * 100, 0, 100);
}

async function refresh_stats() {
  if (!backend_available) {
    // 演示数据：模拟小幅波动的 CPU / 内存
    demo_cpu = clamp(demo_cpu + (Math.random() * 20 - 10), 4, 92);
    const total = 16 * 1024 ** 3;
    stats.value = {
      cpuUsage: Math.round(demo_cpu),
      totalMem: total,
      usedMem: Math.round(total * (0.35 + Math.random() * 0.2)),
      cpuCount: 8,
    };
    return;
  }
  try {
    stats.value = await invoke<SystemStats>("get_system_stats");
    load_error.value = false;
  } catch (e) {
    load_error.value = true;
    console.error("获取系统资源失败:", e);
  }
}

onMounted(() => {
  refresh_stats();
  timer = setInterval(refresh_stats, 1500);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <mdui-card variant="outlined" class="card">
    <div class="card-head">
      <mdui-icon-memory--outlined
        class="card-icon"
      ></mdui-icon-memory--outlined>
      <div>
        <div class="card-title">系统资源</div>
        <div class="card-sub">实时监视系统的运行状态</div>
      </div>
    </div>
    <mdui-divider></mdui-divider>
    <div class="card-body">
      <template v-if="stats">
        <div class="metric">
          <div class="metric-row">
            <span>CPU 占用</span>
            <b>{{ cpu_pct().toFixed(0) }}%</b>
          </div>
          <mdui-linear-progress :value="cpu_pct() / 100"></mdui-linear-progress>
        </div>

        <div class="metric">
          <div class="metric-row">
            <span>内存占用</span>
            <b
              >{{ fmt_bytes(stats.usedMem) }} /
              {{ fmt_bytes(stats.totalMem) }}</b
            >
          </div>
          <mdui-linear-progress :value="mem_pct() / 100"></mdui-linear-progress>
        </div>

        <div class="metric-row">
          <span>内存占用率</span>
          <b>{{ mem_pct().toFixed(0) }}%</b>
        </div>

        <div class="metric-row">
          <span>逻辑核心数</span>
          <b>{{ stats.cpuCount }}</b>
        </div>

        <p v-if="load_error" class="err-text">
          获取系统资源失败，请检查后端连接。
        </p>

        <div class="status-line">
          <span
            class="status-dot"
            :class="backend_available ? 'live' : 'demo'"
          ></span>
          <span>{{
            backend_available
              ? "实时数据，每 1.5 秒刷新"
              : "演示数据（未连接后端）"
          }}</span>
        </div>
      </template>
      <div v-else class="metric-empty">
        <mdui-circular-progress></mdui-circular-progress>
      </div>
    </div>
  </mdui-card>
</template>

<style lang="css" scoped>
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
  font-weight: 600;
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

.metric {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.metric-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.metric-row span {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.metric-row b {
  text-align: right;
}

.metric-empty {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}

.status-line {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.live {
  background: rgb(var(--mdui-color-primary));
}

.status-dot.demo {
  background: rgb(var(--mdui-color-tertiary));
}
</style>
