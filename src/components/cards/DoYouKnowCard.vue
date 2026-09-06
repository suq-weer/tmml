<script setup lang="ts">
import "@mdui/icons/question-mark.js";
import { onBeforeUnmount, onMounted, ref } from "vue";
import { QUIZ } from "../../libs/content/dyn";

const text = ref("");
const lastText = ref("");
let timer: number | undefined;

const randomText = () => {
  let next: string;
  do {
    next = QUIZ[Math.floor(Math.random() * QUIZ.length)];
  } while (next === lastText.value);

  lastText.value = next;
  if (timer !== undefined) window.clearInterval(timer);
  text.value = "";
  let i = 0;
  timer = window.setInterval(() => {
    text.value += next[i++];
    if (i >= next.length) {
      window.clearInterval(timer);
      timer = undefined;
    }
  }, 60);
};

onMounted(randomText);
onBeforeUnmount(() => {
  if (timer !== undefined) window.clearInterval(timer);
});
</script>

<template>
  <mdui-card variant="outlined" class="float-hover-card">
    <div class="card-head">
      <mdui-icon-question-mark class="card-icon"></mdui-icon-question-mark>
      <div class="card-title">你知道吗？</div>
      <div style="flex-grow: 1" />
      <mdui-button @click="randomText">换一个</mdui-button>
    </div>
    <mdui-divider></mdui-divider>
    <div class="quiz-card">
      <p class="quiz-text">{{ text }}</p>
    </div>
  </mdui-card>
</template>

<style scoped>
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
  font-weight: 800;
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

.card-body.flush {
  padding: 4px;
}

.quiz-text {
  display: block;
  margin: 1rem;
  min-height: 2rem;
  font-weight: 800;
}
</style>
