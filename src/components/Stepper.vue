<script lang="ts" setup>
import "@mdui/icons/check.js";
import { computed, nextTick, ref, watch, type StyleValue } from "vue";

defineOptions({ name: "Stepper" });

const props = withDefaults(
  defineProps<{
    steps: string[];
    step?: number;
  }>(),
  { step: -1 },
);

const DOT_SIZE = 30;
const SEGMENT_INSET = DOT_SIZE / 2 + 3;

const root_el = ref<HTMLElement | null>(null);
const track_count = computed(() => Math.max(props.steps.length - 1, 0));

function state_of(i: number): "done" | "active" | "pending" {
  if (props.step >= props.steps.length) return "done";
  if (i < props.step) return "done";
  if (i === props.step) return "active";
  return "pending";
}

function track_style(i: number): StyleValue {
  const n = props.steps.length;
  const left = `${((i + 0.5) / n) * 100}%`;
  const right = `${((n - i - 1.5) / n) * 100}%`;
  return {
    left: `calc(${left} + ${SEGMENT_INSET}px)`,
    right: `calc(${right} + ${SEGMENT_INSET}px)`,
  };
}

function css_var(name: string): string {
  return (
    getComputedStyle(document.documentElement).getPropertyValue(name).trim() ??
    ""
  );
}

function ms_from(var_name: string): number {
  const v = css_var(var_name);
  const n = parseFloat(v);
  return Number.isFinite(n) && n > 0 ? n : 300;
}

let pop_anim: Animation | null = null;

function animate_step_change(idx: number) {
  if (idx < 0 || idx >= props.steps.length) return;
  const host = root_el.value;
  const el = host?.querySelector<HTMLElement>(
    `[data-step="${idx}"] .dot`,
  );
  if (!el) return;
  pop_anim?.cancel();
  pop_anim = el.animate(
    [
      { transform: "scale(.55)", offset: 0 },
      { transform: "scale(1.12)", offset: 0.6 },
      { transform: "scale(1)", offset: 1 },
    ],
    {
      duration: ms_from("--mdui-motion-duration-medium2"),
      easing: css_var("--mdui-motion-easing-emphasized-decelerate") ||
        "cubic-bezier(.05,.7,.1,1)",
    },
  );
}

const CELEBRATE_DURATION = 1100;
const celebrate = ref(false);
let celebrate_timer: number | null = null;

function trigger_celebrate() {
  if (!props.steps.length) return;
  celebrate.value = false;
  if (celebrate_timer) window.clearTimeout(celebrate_timer);
  void nextTick(() => {
    celebrate.value = true;
    celebrate_timer = window.setTimeout(() => {
      celebrate.value = false;
    }, CELEBRATE_DURATION);
  });
}

watch(
  () => props.step,
  (nv, ov) => {
    nextTick(() => animate_step_change(nv));
    if (nv >= props.steps.length && props.steps.length > 0) {
      if (ov >= props.steps.length) return;
      trigger_celebrate();
    } else if (celebrate.value) {
      if (celebrate_timer) window.clearTimeout(celebrate_timer);
      celebrate.value = false;
    }
  },
);
</script>

<template>
  <div v-if="steps.length" ref="root_el" class="stepper">
    <div class="dot-row">
      <span
        v-for="i in track_count"
        :key="'track-' + i"
        class="track"
        :style="track_style(i - 1)"
      >
        <span
          class="track-fill"
          :class="{ 'is-filled': i - 1 < step }"
        ></span>
      </span>

      <div
        v-for="(_, i) in steps"
        :key="'dot-' + i"
        class="cell"
        :data-step="i"
        :class="[
          state_of(i),
          celebrate && state_of(i) === 'done' && i === steps.length - 1
            ? 'celebrate'
            : '',
        ]"
      >
        <span class="celebrate-wave w1"></span>
        <span class="celebrate-wave w2"></span>
        <span class="dot">
          <span v-if="state_of(i) === 'active'" :key="'ring-' + step" class="ring"></span>
          <span class="face">
            <mdui-icon-check class="face-check"></mdui-icon-check>
            <span class="face-num">{{ i + 1 }}</span>
          </span>
        </span>
      </div>
    </div>

    <div class="label-row">
      <div
        v-for="(label, i) in steps"
        :key="'label-' + i"
        class="cell"
        :class="state_of(i)"
      >
        <span class="label">{{ label }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stepper {
  --stepper-dot-size: 30px;
  --stepper-track-size: 4px;
  display: flex;
  flex-direction: column;
  width: 100%;
}

.dot-row {
  position: relative;
  height: var(--stepper-dot-size);
  display: flex;
}

.cell {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.dot {
  position: relative;
  width: var(--stepper-dot-size);
  height: var(--stepper-dot-size);
  border-radius: var(--mdui-shape-corner-full);
  background-color: transparent;
  border: 2px solid rgb(var(--mdui-color-outline-variant));
  color: rgb(var(--mdui-color-on-surface-variant));
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  flex-shrink: 0;
  transition:
    background-color var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard),
    border-color var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard),
    color var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard);
  will-change: transform;
}

.cell.active .dot {
  background-color: rgb(var(--mdui-color-primary));
  border-color: rgb(var(--mdui-color-primary));
  color: rgb(var(--mdui-color-on-primary));
}

.cell.done .dot {
  background-color: rgb(var(--mdui-color-primary));
  border-color: rgb(var(--mdui-color-primary));
  color: rgb(var(--mdui-color-on-primary));
}

.ring {
  position: absolute;
  inset: 0;
  border: 2px solid rgb(var(--mdui-color-primary));
  border-radius: var(--mdui-shape-corner-full);
  pointer-events: none;
  opacity: 0;
  animation: stepper-ring 0.7s
    var(--mdui-motion-easing-standard) forwards;
}

@keyframes stepper-ring {
  0% {
    transform: scale(0.7);
    opacity: 0.55;
  }
  100% {
    transform: scale(1.9);
    opacity: 0;
  }
}

.face {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.face-num {
  font-size: var(--mdui-typescale-label-large-size);
  font-weight: var(--mdui-typescale-label-large-weight);
  line-height: 1;
  font-variant-numeric: tabular-nums;
  transition:
    opacity var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard),
    transform var(--mdui-motion-duration-short3)
      var(--mdui-motion-easing-standard);
}

.face-check {
  position: absolute;
  font-size: calc(var(--stepper-dot-size) * 0.6);
  opacity: 0;
  transform: scale(0.5);
  transition:
    opacity var(--mdui-motion-duration-short4)
      var(--mdui-motion-easing-standard),
    transform var(--mdui-motion-duration-short4)
      var(--mdui-motion-easing-emphasized-decelerate);
}

.cell.done .face-check {
  opacity: 1;
  transform: scale(1);
}

.cell.done .face-num {
  opacity: 0;
  transform: scale(0.5);
}

.cell.celebrate .dot {
  transition: none;
  animation: stepper-celebrate-fill 950ms
    var(--mdui-motion-easing-emphasized-decelerate);
}

.cell.celebrate .face-num {
  animation: stepper-celebrate-num-out 950ms
    var(--mdui-motion-easing-emphasized-accelerate);
}

.cell.celebrate .face-check {
  animation: stepper-celebrate-check-in 950ms
    var(--mdui-motion-easing-emphasized-decelerate);
}

.celebrate-wave {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 6px;
  height: 6px;
  transform: translate(-50%, -50%);
  border: 2px solid rgb(var(--mdui-color-primary));
  border-radius: var(--mdui-shape-corner-full);
  box-sizing: border-box;
  pointer-events: none;
  opacity: 0;
}

.cell.celebrate .celebrate-wave {
  animation: stepper-celebrate-wave 900ms
    var(--mdui-motion-easing-standard) forwards;
}

.cell.celebrate .celebrate-wave.w2 {
  animation-delay: 150ms;
}

@keyframes stepper-celebrate-wave {
  0% {
    width: 6px;
    height: 6px;
    opacity: 0.9;
  }
  70% {
    opacity: 0.5;
  }
  100% {
    width: 168px;
    height: 168px;
    opacity: 0;
  }
}

@keyframes stepper-celebrate-fill {
  0% {
    background-color: transparent;
    border-color: rgb(var(--mdui-color-outline-variant));
    transform: scale(0.9);
  }
  25% {
    background-color: rgb(var(--mdui-color-primary));
    border-color: rgb(var(--mdui-color-primary));
    transform: scale(1.5);
  }
  42% {
    transform: scale(0.92);
  }
  58% {
    transform: scale(1.24);
  }
  72%,
  100% {
    background-color: rgb(var(--mdui-color-primary));
    border-color: rgb(var(--mdui-color-primary));
    transform: scale(1);
  }
}

@keyframes stepper-celebrate-num-out {
  0%,
  10% {
    opacity: 1;
    transform: scale(1);
  }
  100% {
    opacity: 0;
    transform: scale(0.5);
  }
}

@keyframes stepper-celebrate-check-in {
  0%,
  24% {
    opacity: 0;
    transform: scale(0.1) rotate(-30deg);
  }
  46% {
    opacity: 1;
    transform: scale(1.25) rotate(8deg);
  }
  62% {
    transform: scale(0.92) rotate(-4deg);
  }
  78%,
  100% {
    opacity: 1;
    transform: scale(1) rotate(0);
  }
}

.track {
  position: absolute;
  top: calc(
    (var(--stepper-dot-size) - var(--stepper-track-size)) / 2
  );
  height: var(--stepper-track-size);
  border-radius: var(--mdui-shape-corner-full);
  background-color: rgb(var(--mdui-color-surface-container-highest));
  overflow: hidden;
  pointer-events: none;
}

.track-fill {
  display: block;
  width: 100%;
  height: 100%;
  background-color: rgb(var(--mdui-color-primary));
  border-radius: inherit;
  transform: scaleX(0);
  transform-origin: left center;
  transition: transform var(--mdui-motion-duration-medium2)
    var(--mdui-motion-easing-standard);
}

.track-fill.is-filled {
  transform: scaleX(1);
}

.label-row {
  display: flex;
  margin-top: 8px;
}

.label {
  display: block;
  max-width: 100%;
  font-size: var(--mdui-typescale-label-large-size);
  font-weight: var(--mdui-typescale-label-large-weight);
  line-height: var(--mdui-typescale-label-large-line-height);
  letter-spacing: var(--mdui-typescale-label-large-tracking);
  text-align: center;
  color: rgb(var(--mdui-color-on-surface-variant));
  overflow-wrap: break-word;
  transition: color var(--mdui-motion-duration-short3)
    var(--mdui-motion-easing-standard);
}

.cell.active .label {
  color: rgb(var(--mdui-color-primary));
  font-weight: 600;
}

.cell.done .label {
  color: rgb(var(--mdui-color-on-surface-variant));
}
</style>
