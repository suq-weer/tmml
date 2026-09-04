<script setup lang="ts">
import "@mdui/icons/question-mark.js";
import { onBeforeUnmount, onMounted, ref } from "vue";

const quiz = [
  // 启动器相关
  "目前启动器仍然将愚人节版本当快照版处理……",
  "我相信你肯定会把玩启动器里会伸缩的标题栏（",
  "本启动器原型最初是 Electron 技术栈，后面想学 Rust 了才转 Tauri 的",
  "启动器的 AppImage 版本受 Tauri 影响其实并不能正常运行",
  "这个卡片以后可以自定义位置……大概吧",
  "Never gonna give you up.",
  "这个启动器是 Xiaosu 第一个能启动 Minecraft 的启动器（",
  "启动实例后的灵动岛好不好看？虽然一台电脑开不了那么多 Minecraft……",
  "init repo 前这个项目断断续续写了半个月……XD",
  // Minecraft 小知识
  "Mojang 在 26.3 Pre 1 中重新加入了边境之地",
  // MC 社区知识
  "Neoforge 是 Forge 团队分裂出来的项目",
  "Fabric 比 Forge 更年轻",
];

const text = ref("");
const lastText = ref("");
let timer: number | undefined;

const randomText = () => {
  let next: string;
  do {
    next = quiz[Math.floor(Math.random() * quiz.length)];
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
