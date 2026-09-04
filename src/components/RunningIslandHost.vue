<script lang="ts" setup>
import "@mdui/icons/stop.js";
import "@mdui/icons/close.js";
import "@mdui/icons/keyboard-arrow-down.js";
import {
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import MCIcon from "../assets/mc_icon.png";
import { renderLogLine } from "../libs/logfmt";
import {
  RunningSession,
  RunningStatus,
  init_session_events,
  dispose_session_events,
  useRunningStore,
} from "../libs/running";

const {
  sessions,
  expandedId,
  dockClearance,
  logsBySid,
  focus_session,
  collapse,
  stop_session,
  destroy_session,
} = useRunningStore();

// ---- 几何常量（与 CSS 中 dock 位置保持一致） ----
const PANEL_MARGIN = 14;
const DOCK_BOTTOM = 18; // .run-dock bottom
const FAB_H = 56;
const PANEL_INSET_BOTTOM = DOCK_BOTTOM + FAB_H + 16;
const PANEL_RADIUS = 28;
const TOAST_MAX_W = 360;

const vp = reactive({ w: window.innerWidth, h: window.innerHeight });
const overlayOn = ref(false);
const contentSession = ref<RunningSession | null>(null);
/** 置顶/重排瞬间临时关闭 Dock FLIP，保证按最终布局测量胶囊 rect */
const dockFreeze = ref(false);

const backdropEl = ref<HTMLElement | null>(null);
const panelEl = ref<HTMLElement | null>(null);
const innerEl = ref<HTMLElement | null>(null);
const logAreaEl = ref<HTMLElement | null>(null);
const dockMeasEl = ref<HTMLElement | null>(null);
const pillEls = new Map<number, HTMLElement>();

let panelAnim: Animation | null = null;
let backdropAnim: Animation | null = null;

function cancelAllAnims() {
  panelAnim?.cancel();
  panelAnim = null;
  backdropAnim?.cancel();
  backdropAnim = null;
  const p = panelEl.value;
  if (p) {
    p.style.transform = "";
    p.style.borderRadius = `${PANEL_RADIUS}px`;
    p.style.opacity = "1";
    p.style.transition = "";
  }
  const bd = backdropEl.value;
  if (bd) {
    bd.style.opacity = "1";
    bd.style.transition = "";
  }
}

function setPillRef(id: number, el: unknown) {
  if (el) pillEls.set(id, el as HTMLElement);
  else pillEls.delete(id);
}

function sessionById(id: number) {
  return sessions.value.find((s) => s.id === id);
}

function currentPanelRect() {
  return {
    left: PANEL_MARGIN,
    top: PANEL_MARGIN,
    width: vp.w - PANEL_MARGIN * 2,
    height: vp.h - PANEL_MARGIN - PANEL_INSET_BOTTOM,
  };
}

function pillRectOf(id: number) {
  const el = pillEls.get(id);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, width: r.width, height: r.height };
}

/** 将「按最终几何定位的面板」映射到 target 矩形的 transform */
function morphTo(target: {
  left: number;
  top: number;
  width: number;
  height: number;
}) {
  const pr = currentPanelRect();
  const sx = target.width / pr.width;
  const sy = target.height / pr.height;
  const dx = target.left + target.width / 2 - (pr.left + pr.width / 2);
  const dy = target.top + target.height / 2 - (pr.top + pr.height / 2);
  return `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})`;
}

function waitFrames(n = 1): Promise<void> {
  return new Promise((resolve) => {
    let left = n;
    const step = () => {
      if (left <= 0) resolve();
      else {
        left--;
        requestAnimationFrame(step);
      }
    };
    requestAnimationFrame(step);
  });
}

/** 面板挂载后：先同步到起始态（避免闪现），再返回 */
async function mountPanel(
  startTransform: string | null,
  startRadius: number,
  startOpacity: number,
) {
  overlayOn.value = true;
  await nextTick();
  const p = panelEl.value;
  if (p) {
    p.style.transition = "none";
    p.style.transform = startTransform ?? "none";
    p.style.borderRadius = `${startRadius}px`;
    p.style.opacity = String(startOpacity);
  }
  if (backdropEl.value) {
    backdropEl.value.style.transition = "none";
    backdropEl.value.style.opacity = String(startOpacity);
  }
  await waitFrames(2);
}

/** 面板可见后，跑一段 WAAPI，结束后收敛 inline 状态 */
function animatePanel(
  keyframes: Keyframe[],
  opts: KeyframeAnimationOptions,
): Promise<void> {
  const p = panelEl.value;
  if (!p) return Promise.resolve();
  panelAnim = p.animate(keyframes, { ...opts, fill: "forwards" });
  return panelAnim.finished
    .then(() => {
      p.style.opacity = "1";
      p.style.transform = "";
      p.style.borderRadius = `${PANEL_RADIUS}px`;
    })
    .catch(() => undefined);
}

function animateBackdrop(opacity: number, duration = 240) {
  const bd = backdropEl.value;
  if (!bd) return Promise.resolve();
  backdropAnim = bd.animate(
    [{ opacity: bd.style.opacity || "0" }, { opacity }],
    { duration, easing: "ease-out", fill: "forwards" },
  );
  return backdropAnim.finished
    .then(() => {
      bd.style.opacity = String(opacity);
      bd.style.transition = "";
    })
    .catch(() => undefined);
}

function setInner(opacity: number, delay = 0, duration = 220) {
  if (innerEl.value) {
    innerEl.value.style.transition = `opacity ${duration}ms ease ${delay}ms`;
    innerEl.value.style.opacity = String(opacity);
  }
}

// 日志/状态全部由 running.ts 订阅的后端事件驱动，此处不再有模拟内容。

// ---- 动画操作串行队列：保证展开/切换/收起不并发，避免状态互相踩踏 ----
let opTail: Promise<void> = Promise.resolve();

function enqueueOp(fn: () => Promise<void>): void {
  const run = opTail.then(() => fn());
  opTail = run.catch(() => undefined);
}

// ---- 展开 / 收起 / 切换 ----
async function openFresh(session: RunningSession) {
  // 串行队列内执行；若期间目标已变化则放弃本次
  if (expandedId.value !== session.id) return;
  cancelAllAnims();
  contentSession.value = session;
  setInner(0, 0, 0);
  if (!overlayOn.value) {
    // 无遮罩：整层淡入 + 轻微缩放
    await mountPanel(null, PANEL_RADIUS, 0);
    animateBackdrop(1, 260);
    await animatePanel(
      [
        { transform: "scale(.94) translateY(2vh)", opacity: 0, offset: 0 },
        { transform: "scale(1) translateY(0)", opacity: 1, offset: 1 },
      ],
      { duration: 320, easing: "cubic-bezier(.32,.72,.24,1)" },
    );
  } else {
    // 已在展开态：直接切换为新的会话内容
    await waitFrames(1);
    const p = panelEl.value;
    if (p) {
      panelAnim = p.animate(
        [
          { transform: "scale(.98)", opacity: 0.6, offset: 0 },
          { transform: "scale(1)", opacity: 1, offset: 1 },
        ],
        {
          duration: 240,
          easing: "cubic-bezier(.32,.72,.24,1)",
          fill: "forwards",
        },
      );
    }
  }
  setInner(1, 30);
}

async function openFromPill(session: RunningSession) {
  if (expandedId.value !== session.id) return;
  cancelAllAnims();
  contentSession.value = session;
  setInner(0, 0, 0);
  // 置顶刚发生：等待 Dock 按最终布局渲染后再测量（dockFreeze 由 onPillClick 定时解除）
  if (dockFreeze.value) {
    await nextTick();
    await waitFrames(2);
  }
  const rect = pillRectOf(session.id) ?? currentPanelRect();
  await mountPanel(morphTo(rect), rect.height / 2, 0);
  animateBackdrop(1, 320);
  await animatePanel(
    [
      {
        transform: morphTo(rect),
        borderRadius: `${rect.height / 2}px`,
        opacity: 0.4,
        offset: 0,
      },
      {
        transform: "none",
        borderRadius: `${PANEL_RADIUS}px`,
        opacity: 1,
        offset: 1,
      },
    ],
    { duration: 400, easing: "cubic-bezier(.32,.72,.24,1)" },
  );
  setInner(1, 30);
}

async function switchTo(session: RunningSession) {
  if (expandedId.value !== session.id) return;
  cancelAllAnims();
  if (dockFreeze.value) {
    await nextTick();
    await waitFrames(2);
  }
  const rect = pillRectOf(session.id) ?? currentPanelRect();
  const old = contentSession.value;
  const p = panelEl.value;
  if (!old || !p) {
    contentSession.value = session;
    setInner(1, 20);
    return;
  }
  // 1) 当前面板收缩到目标胶囊
  setInner(0, 0, 120);
  await animatePanel(
    [
      {
        transform: "none",
        borderRadius: `${PANEL_RADIUS}px`,
        opacity: 1,
        offset: 0,
      },
      {
        transform: morphTo(rect),
        borderRadius: `${rect.height / 2}px`,
        opacity: 0.6,
        offset: 1,
      },
    ],
    { duration: 200, easing: "cubic-bezier(.4,0,.2,1)" },
  );
  // 2) 内容换新后重新展开
  contentSession.value = session;
  setInner(0, 0, 0);
  await animatePanel(
    [
      {
        transform: morphTo(rect),
        borderRadius: `${rect.height / 2}px`,
        opacity: 0.4,
        offset: 0,
      },
      {
        transform: "none",
        borderRadius: `${PANEL_RADIUS}px`,
        opacity: 1,
        offset: 1,
      },
    ],
    { duration: 340, easing: "cubic-bezier(.32,.72,.24,1)" },
  );
  setInner(1, 20);
}

async function requestCollapse() {
  if (!overlayOn.value) return;
  cancelAllAnims();
  const s = contentSession.value;
  const p = panelEl.value;
  if (!p) {
    overlayOn.value = false;
    contentSession.value = null;
    collapse();
    return;
  }
  setInner(0, 0, 120);
  const rect = s && pillRectOf(s.id) ? pillRectOf(s.id)! : currentPanelRect();
  // 遮罩同步淡出；面板收缩到胶囊后立即卸载（同一微任务内，无重绘闪烁）
  animateBackdrop(0, 180);
  await animatePanel(
    [
      {
        transform: "none",
        borderRadius: `${PANEL_RADIUS}px`,
        opacity: 1,
        offset: 0,
      },
      {
        transform: morphTo(rect),
        borderRadius: `${rect.height / 2}px`,
        opacity: 0.5,
        offset: 1,
      },
    ],
    { duration: 300, easing: "cubic-bezier(.4,0,.2,1)" },
  );
  overlayOn.value = false;
  contentSession.value = null;
  collapse();
}

function queueCollapse() {
  enqueueOp(() => requestCollapse());
}

function scheduleDockUnfreeze() {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      dockFreeze.value = false;
    });
  });
}

function onPillClick(session: RunningSession) {
  if (expandedId.value === session.id && overlayOn.value) {
    queueCollapse();
    return;
  }
  // 置顶：先冻结 Dock 过渡，等重排后的布局稳定再展开，避免 FLIP 干扰 morph 起点
  dockFreeze.value = true;
  scheduleDockUnfreeze();
  focus_session(session.id);
}

/** 终止运行中的会话：通知后端杀进程，面板保留，等 exited 事件显示“已停止” */
function onStop(session: RunningSession) {
  stop_session(session.id);
}

/** terminal 状态（已退出/启动失败）下，右上角只保留“销毁面板” */
function onDestroy(session: RunningSession) {
  destroy_session(session.id);
  overlayOn.value = false;
  contentSession.value = null;
}

// 队列执行时按「当时」状态决定形态：fresh=直接展开；无遮罩=从胶囊展开；有遮罩且目标变化=切换
async function driveExpanded(id: number) {
  if (expandedId.value !== id) return;
  const s = sessionById(id);
  if (!s) return;
  if (s.freshOpen) {
    s.freshOpen = false;
    await openFresh(s);
    return;
  }
  if (!overlayOn.value) await openFromPill(s);
  else await switchTo(s);
}

watch(expandedId, async (nv) => {
  if (nv == null) return;
  enqueueOp(() => driveExpanded(nv));
});

// ---- Toast 避让 ----
function updateDockClearance() {
  if (overlayOn.value) {
    dockClearance.value = 0;
    return;
  }
  const el = dockMeasEl.value;
  if (!el || sessions.value.length === 0) {
    dockClearance.value = 0;
    return;
  }
  const rect = el.getBoundingClientRect();
  const toastLeft = vp.w - 16 - Math.min(TOAST_MAX_W, vp.w - 32);
  if (rect.right > toastLeft) {
    dockClearance.value = Math.max(16, vp.h - rect.top + 8);
  } else {
    dockClearance.value = 0;
  }
}

function onResize() {
  vp.w = window.innerWidth;
  vp.h = window.innerHeight;
  updateDockClearance();
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && overlayOn.value) queueCollapse();
}

let dockResizeObs: ResizeObserver | null = null;

function attachDockObserver() {
  if (!dockMeasEl.value) return;
  if (dockResizeObs) dockResizeObs.disconnect();
  dockResizeObs = new ResizeObserver(() => updateDockClearance());
  dockResizeObs.observe(dockMeasEl.value);
}

watch(dockMeasEl, () => {
  nextTick(() => {
    attachDockObserver();
    updateDockClearance();
  });
});

watch(overlayOn, () => {
  nextTick(() => updateDockClearance());
});

watch(
  () => sessions.value.length,
  () => {
    nextTick(() => {
      updateDockClearance();
      attachDockObserver();
    });
  },
);

// 日志滚动到底部：当前展开会话的新日志到达时自动跟随
watch(
  () => {
    const s = contentSession.value;
    return s ? (logsBySid[s.id]?.length ?? 0) : 0;
  },
  () => {
    if (!overlayOn.value) return;
    const area = logAreaEl.value;
    if (area) area.scrollTop = area.scrollHeight;
  },
);

onMounted(async () => {
  window.addEventListener("resize", onResize);
  window.addEventListener("keydown", onKey);
  await init_session_events().catch((e) =>
    console.error("订阅会话事件失败:", e),
  );
  nextTick(() => {
    attachDockObserver();
    updateDockClearance();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", onResize);
  window.removeEventListener("keydown", onKey);
  dockResizeObs?.disconnect();
  panelAnim?.cancel();
  dispose_session_events();
});

// ---- 展示辅助 ----
function statusText(s: RunningStatus) {
  switch (s) {
    case "running":
      return "运行中";
    case "launching":
      return "启动中";
    case "error":
      return "启动失败";
    default:
      return "已停止";
  }
}

/** 是否已结束（正常/异常退出或启动失败）：此时面板右上角只提供“销毁面板” */
function isTerminal(s: RunningStatus) {
  return s === "exited" || s === "error";
}

function statusClass(s: RunningStatus) {
  if (isTerminal(s)) return s === "error" ? "st-error" : "st-exited";
  return s === "running" ? "st-running" : "st-launching";
}

function pillDotClass(s: RunningStatus) {
  return isTerminal(s)
    ? "dot-exited"
    : s === "running"
      ? "dot-running"
      : "dot-launching";
}

function iconOf(session: RunningSession) {
  return session.instance.icon || MCIcon;
}

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function fmtElapsed(ms: number) {
  const sec = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  if (h > 0) return `${h} 时 ${pad2(m)} 分`;
  return `${pad2(m)} 分 ${pad2(s)} 秒`;
}

/** 状态末行文案：运行中显示运行时长，结束时展示退出/失败原因 */
function statusHint(s: RunningSession) {
  if (s.status === "error") return s.message ?? "启动失败";
  if (s.status === "exited")
    return s.message ?? `已退出（代码 ${s.exitCode ?? "?"}）`;
  if (s.status === "running") return "正在实时捕获游戏日志…";
  return "正在准备环境并构建启动命令…";
}
</script>

<template>
  <div>
    <!-- 展开态：全窗圆角面板 + 全局遮罩 -->
    <div v-if="overlayOn" class="island-overlay">
      <div
        ref="backdropEl"
        class="island-backdrop"
        @click="queueCollapse"
      ></div>
      <section
        ref="panelEl"
        class="island-panel"
        :style="{
          left: PANEL_MARGIN + 'px',
          top: PANEL_MARGIN + 'px',
          width: vp.w - PANEL_MARGIN * 2 + 'px',
          height: vp.h - PANEL_MARGIN - PANEL_INSET_BOTTOM + 'px',
          borderRadius: PANEL_RADIUS + 'px',
        }"
        @click.stop
      >
        <div ref="innerEl" class="island-inner">
          <template v-if="contentSession">
            <header class="island-head">
              <img
                class="island-avatar"
                :src="iconOf(contentSession)"
                alt="avatar"
              />
              <div class="island-title-wrap">
                <div class="island-title">
                  {{ contentSession.instance.name }}
                </div>
                <div class="island-sub">
                  {{ contentSession.instance.versionId }}
                </div>
              </div>
              <span
                class="status-inner"
                :class="statusClass(contentSession.status)"
              >
                <span class="status-dot"></span>
                {{ statusText(contentSession.status) }}
              </span>
              <div class="island-actions">
                <template v-if="isTerminal(contentSession.status)">
                  <mdui-button-icon
                    title="销毁面板"
                    @click="onDestroy(contentSession)"
                  >
                    <mdui-icon-close></mdui-icon-close>
                  </mdui-button-icon>
                </template>
                <template v-else>
                  <mdui-button-icon title="收起为胶囊" @click="queueCollapse">
                    <mdui-icon-keyboard-arrow-down></mdui-icon-keyboard-arrow-down>
                  </mdui-button-icon>
                  <mdui-button-icon
                    title="结束会话"
                    @click="onStop(contentSession)"
                  >
                    <mdui-icon-stop></mdui-icon-stop>
                  </mdui-button-icon>
                </template>
              </div>
            </header>

            <div class="island-stats">
              <div class="stat">
                <div class="stat-num">{{ contentSession.pid ?? "—" }}</div>
                <div class="stat-label">进程 PID</div>
              </div>
              <div class="stat">
                <div class="stat-num">
                  {{
                    contentSession.status === "running"
                      ? "存活"
                      : (contentSession.exitCode ?? "—")
                  }}
                </div>
                <div class="stat-label">退出代码</div>
              </div>
              <div class="stat">
                <div class="stat-num">
                  {{ fmtElapsed(Date.now() - contentSession.launchedAt) }}
                </div>
                <div class="stat-label">已运行</div>
              </div>
            </div>

            <div class="island-log">
              <div class="log-head">
                <span>守护监视 · 运行日志</span>
                <span class="log-head-sub">{{
                  statusHint(contentSession)
                }}</span>
              </div>
              <div ref="logAreaEl" class="log-body">
                <p
                  v-for="(line, i) in logsBySid[contentSession.id] ?? []"
                  :key="i"
                  class="log-line"
                >
                  <code v-html="renderLogLine(line.text, line.kind)" />
                </p>
                <p
                  v-if="(logsBySid[contentSession.id] ?? []).length === 0"
                  class="log-empty"
                >
                  暂无输出，等待会话数据…
                </p>
              </div>
            </div>
          </template>
        </div>
      </section>
    </div>

    <!-- 收缩态：底部居中的 FAB 胶囊群 -->
    <div v-if="sessions.length" class="run-dock">
      <div ref="dockMeasEl" class="run-dock-wrap">
        <transition-group
          name="dock"
          tag="div"
          :class="['run-dock-row', { 'dock-frozen': dockFreeze }]"
        >
          <mdui-fab
            v-for="(s, i) in sessions"
            :key="s.id"
            :ref="(el: unknown) => setPillRef(s.id, el)"
            :extended="i === 0"
            :class="{ 'is-active': expandedId === s.id && overlayOn }"
            class="run-pill"
            @click="onPillClick(s)"
          >
            <img slot="icon" class="pill-icon" :src="iconOf(s)" alt="icon" />
            <template v-if="i === 0">
              <span class="pill-label">
                <span class="pill-name">{{ s.instance.name }}</span>
                <span class="pill-status">
                  <span class="pill-dot" :class="pillDotClass(s.status)"></span>
                  {{ statusText(s.status) }}
                </span>
              </span>
            </template>
          </mdui-fab>
        </transition-group>
      </div>
    </div>
  </div>
</template>

<style scoped>
.island-overlay {
  position: fixed;
  inset: 0;
  z-index: 2200;
  pointer-events: none;
}

.island-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(var(--mdui-color-scrim), 0.45);
  opacity: 0;
  pointer-events: auto;
}

.island-panel {
  position: absolute;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: rgb(var(--mdui-color-surface-container));
  color: rgb(var(--mdui-color-on-surface));
  box-shadow: 0 24px 48px rgba(var(--mdui-color-scrim), 0.35);
  pointer-events: auto;
  opacity: 0;
  will-change: transform, border-radius, opacity;
  transform-origin: center;
}

.island-inner {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 14px 22px 16px;
  opacity: 1;
}

.island-head {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-bottom: 14px;
  border-bottom: 1px solid rgb(var(--mdui-color-outline-variant));
}

.island-avatar {
  width: 52px;
  height: 52px;
  border-radius: var(--mdui-shape-corner-medium);
  object-fit: cover;
  image-rendering: pixelated;
  flex-shrink: 0;
}

.island-title-wrap {
  flex: 1;
  min-width: 0;
}

.island-title {
  font-size: 22px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.island-sub {
  font-size: 13px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.island-actions {
  display: flex;
  gap: 4px;
  margin-left: 4px;
}

.status-inner {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.st-running {
  color: rgb(var(--mdui-color-tertiary));
}

.st-launching {
  color: rgb(var(--mdui-color-secondary));
}

.st-exited {
  color: rgb(var(--mdui-color-on-surface-variant));
}

.st-error {
  color: rgb(var(--mdui-color-error));
}

.st-running .status-dot {
  background: rgb(var(--mdui-color-tertiary));
}

.st-launching .status-dot {
  background: rgb(var(--mdui-color-secondary));
  animation: dock-blink 0.7s ease-in-out infinite;
}

.st-exited .status-dot {
  background: rgb(var(--mdui-color-outline));
}

.st-error .status-dot {
  background: rgb(var(--mdui-color-error));
  animation: dock-blink 0.7s ease-in-out infinite;
}

.island-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  padding: 16px 0;
}

.stat {
  background: rgb(var(--mdui-color-surface-container-high));
  border: 1px solid rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-medium);
  padding: 12px 16px;
}

.stat-num {
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.stat-label {
  font-size: 12px;
  margin-top: 2px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.island-log {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: rgb(var(--mdui-color-surface-container-low));
  border: 1px solid rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-medium);
  overflow: hidden;
}

.log-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-weight: 600;
  font-size: 13px;
  padding: 8px 14px;
  border-bottom: 1px solid rgb(var(--mdui-color-outline-variant));
}

.log-head-sub {
  font-weight: 400;
  font-size: 12px;
  color: rgb(var(--mdui-color-on-surface-variant));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 6px 14px 12px;
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
  font-size: 12.5px;
  line-height: 1.65;
}

.log-line {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
  word-break: break-word;
}

.log-empty {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
  opacity: 0.6;
  font-style: italic;
}

.island-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 12px;
}

.foot-hint {
  font-size: 12px;
  color: rgb(var(--mdui-color-on-surface-variant));
}

/* 收缩态 Dock */
.run-dock {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 18px;
  display: flex;
  justify-content: center;
  z-index: 2400;
  pointer-events: none;
}

.run-dock-wrap {
  pointer-events: auto;
  display: inline-flex;
  max-width: calc(100vw - 40px);
}

.run-dock-row {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding-bottom: 6px;
}

.run-pill {
  flex-shrink: 0;
  --mdui-shape-corner-large: 28px;
}

.run-pill.is-active {
  --mdui-color-primary-container: var(--mdui-color-secondary-container);
  --mdui-color-on-primary-container: var(--mdui-color-on-secondary-container);
}

.pill-icon {
  width: 24px;
  height: 24px;
  border-radius: 4px;
  object-fit: cover;
  image-rendering: pixelated;
}

.pill-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 240px;
  min-width: 0;
}

.pill-name {
  font-weight: 600;
  font-size: 13px;
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pill-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  opacity: 0.88;
  flex-shrink: 0;
}

.pill-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-running {
  background: rgb(var(--mdui-color-tertiary));
}

.dot-launching {
  background: rgb(var(--mdui-color-error));
  animation: dock-blink 0.7s ease-in-out infinite;
}

.dot-exited {
  background: rgb(var(--mdui-color-outline));
}

@keyframes dock-blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.25;
  }
}

.dock-enter-active,
.dock-leave-active,
.dock-move {
  transition: all 0.3s var(--mdui-motion-easing-standard);
}

.dock-frozen .dock-enter-active,
.dock-frozen .dock-leave-active,
.dock-frozen .dock-move {
  transition: none;
}

.dock-enter-from {
  opacity: 0;
  transform: scale(0.6) translateY(14px);
}

.dock-leave-to {
  opacity: 0;
  transform: scale(0.6);
}

.dock-leave-active {
  position: absolute;
}
</style>
