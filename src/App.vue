<script setup lang="ts">
import { RouterView, useRouter } from "vue-router";
import { computed, onMounted, ref, watch } from "vue";
import ToastHost from "./components/ToastHost.vue";
import RunningIslandHost from "./components/RunningIslandHost.vue";
import { useToastStore } from "./libs/toast";
import {
  get_instance_icon,
  get_profile_avatar,
  useProfileStore,
} from "./libs/profile";
import { launch_backend } from "./libs/running";
import MCIcon from "./assets/mc_icon.png";
import SteveIcon from "./assets/steve.png";
import "@mdui/icons/notifications";
import "@mdui/icons/group.js";
import "@mdui/icons/checkroom.js";
import "@mdui/icons/settings.js";
import "@mdui/icons/play-circle.js";
import "@mdui/icons/person-add-alt-1.js";

const router = useRouter();
const { notifications, pushToast } = useToastStore();
const { current, lastLaunched, profiles, refresh } = useProfileStore();

const unread_count = computed(() => notifications.value.length);
const has_profiles = computed(() => profiles.value.length > 0);
const transitionName = ref<"push" | "pop">("push");

let navigationDirection: "push" | "pop" = "push";

router.options.history.listen((_to, _from, info) => {
  navigationDirection = info.delta < 0 ? "pop" : "push";
});

router.afterEach(() => {
  transitionName.value = navigationDirection;
  navigationDirection = "push";
});

const current_profile_name = computed(
  () => current.value?.name ?? profiles.value[0]?.name ?? "",
);

const instance_icon_src = ref(MCIcon);
const avatar_src = ref(SteveIcon);

async function update_avatar() {
  const name = current.value?.name;
  if (!name) {
    avatar_src.value = SteveIcon;
    return;
  }
  try {
    const data = await get_profile_avatar(name);
    avatar_src.value = data ?? MCIcon;
  } catch (e) {
    console.error("获取皮肤头像失败", e);
    pushToast({
      level: "error",
      title: "获取玩家 " + name + " 的皮肤头像失败",
    });
    avatar_src.value = SteveIcon;
  }
}

function push(route: string) {
  router.push(route);
}

function relaunch_last() {
  const last = lastLaunched.value;
  if (!last) {
    pushToast({
      level: "warning",
      title: "最近没有运行任何实例，请先在实例列表启动一个",
      message: "注意：此 Toast 出现可能是一个未预料的现象",
    });
    return;
  }
  launch_backend({
    id: last.versionId,
    name: last.name,
    versionId: last.versionId,
    path: `versions/${last.dir}`,
    icon: instance_icon_src.value,
  }).catch((e) => {
    console.error("继续启动失败:", e);
    pushToast({ level: "error", title: "启动失败", message: String(e) });
  });
}

watch(current, update_avatar, { immediate: true });

onMounted(async () => {
  await refresh();
  if (lastLaunched.value) {
    const icon = await get_instance_icon(lastLaunched.value.dir);
    if (icon) instance_icon_src.value = icon;
  }
});
</script>

<template>
  <mdui-layout class="div">
    <mdui-top-app-bar
      variant="large"
      class="title"
      scroll-behavior="shrink elevate"
      scroll-target=".main-page"
    >
      <img class="mc-avatar" :src="avatar_src" alt="avatar" />
      <mdui-top-app-bar-title>
        Too Many Minecraft Launcher
        <mdui-card slot="label-large" class="label-large-content">
          <template v-if="has_profiles">
            <div class="label">
              欢迎！{{ current_profile_name }}
              <div style="flex-grow: 1" />
              <mdui-button @click="push('/profiles')" variant="elevated">
                <mdui-icon-group slot="icon"></mdui-icon-group>
                档案管理
              </mdui-button>
              <mdui-button
                @click="
                  pushToast({ level: 'info', title: '皮肤更换功能敬请期待~' })
                "
                variant="elevated"
              >
                <mdui-icon-checkroom slot="icon"></mdui-icon-checkroom>
                更换皮肤
              </mdui-button>
            </div>
          </template>
          <template v-else>
            <span class="create_text">请新建一个用户档案</span>
            <mdui-button @click="push('/profiles')" variant="elevated">
              <mdui-icon-person-add-alt-1
                slot="icon"
              ></mdui-icon-person-add-alt-1>
              新建档案
            </mdui-button>
          </template>
          <sub class="last-launched">
            <template v-if="lastLaunched">
              <div class="label">
                <div>
                  从实例
                  <img
                    class="instance_icon"
                    :src="instance_icon_src"
                    alt="instance_icon"
                  />
                  {{ lastLaunched.name }} 继续
                </div>
                <div style="flex-grow: 1" />
                <mdui-button @click="relaunch_last()" variant="elevated">
                  <mdui-icon-play-circle slot="icon"></mdui-icon-play-circle>
                  运行实例
                </mdui-button>
              </div>
            </template>
            <template v-else>
              最近没有运行任何实例，请先在实例列表启动一个
            </template>
          </sub>
        </mdui-card>
      </mdui-top-app-bar-title>
      <div style="flex-grow: 1"></div>
      <div class="notif-wrap">
        <mdui-button-icon>
          <mdui-icon-settings></mdui-icon-settings>
        </mdui-button-icon>
        <mdui-button-icon @click="push('/notifications')">
          <mdui-icon-notifications></mdui-icon-notifications>
        </mdui-button-icon>
        <mdui-badge v-if="unread_count > 0" class="notif-badge">{{
          unread_count
        }}</mdui-badge>
      </div>
    </mdui-top-app-bar>
    <mdui-layout-main class="main-page">
      <div class="route">
        <RouterView v-slot="{ Component, route: viewRoute }">
          <Transition :name="transitionName">
            <component
              :is="Component"
              :key="viewRoute.matched[0]?.path ?? viewRoute.path"
            />
          </Transition>
        </RouterView>
      </div>
    </mdui-layout-main>
  </mdui-layout>
  <ToastHost />
  <RunningIslandHost />
</template>

<style lang="css">
.div {
  height: 100%;
  position: relative;
  overflow: hidden;
}
.main-page {
  min-height: 100%;
  overflow: auto;
}
.route {
  position: relative;
  min-height: 100vh;
  overflow-x: clip;
}
.push-enter-active,
.push-leave-active,
.pop-enter-active,
.pop-leave-active {
  transition:
    transform var(--mdui-motion-duration-long2)
      var(--mdui-motion-easing-emphasized),
    opacity var(--mdui-motion-duration-long2)
      var(--mdui-motion-easing-emphasized);
}
.push-enter-active {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
}
.push-enter-from {
  transform: translateX(150%);
}
.push-leave-to {
  transform: translateX(-150%);
  opacity: 0;
}
.pop-leave-active {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
}
.pop-enter-from {
  transform: translateX(-150%);
  opacity: 0;
}
.pop-leave-to {
  transform: translateX(150%);
}
mdui-top-app-bar[variant="large"] {
  height: 20rem !important;
}
mdui-top-app-bar[variant="large"][shrink]:not([shrink="false" i]) {
  height: 4rem !important;
}
.mc-avatar {
  width: 8rem;
  height: 8rem;
  margin-left: 1rem !important;
  aspect-ratio: 1 / 1;
  object-fit: cover;
  image-rendering: pixelated;
  border-radius: var(--mdui-shape-corner-medium);
  margin-top: 1rem;
  flex-shrink: 0;
  box-shadow: var(--mdui-elevation-level5);
  transition:
    width var(--mdui-motion-duration-short4) var(--mdui-motion-easing-standard),
    height var(--mdui-motion-duration-short4) var(--mdui-motion-easing-standard),
    border-radius var(--mdui-motion-duration-short4)
      var(--mdui-motion-easing-standard),
    margin var(--mdui-motion-duration-short4) var(--mdui-motion-easing-standard);
}
mdui-top-app-bar[shrink]:not([shrink="false" i]) .mc-avatar {
  width: 1.875rem;
  height: 1.875rem;
  margin-left: 0 !important;
  margin-top: 5px !important;
  border-radius: 20px;
}
.notif-wrap {
  position: relative;
  display: inline-flex;
}
.notif-badge {
  position: absolute;
  top: -2px;
  right: -4px;
}
.instance_icon {
  height: 1rem;
}
.create_text {
  color: rgb(var(--mdui-color-error));
}
.label-large-content {
  display: flexbox;
  background-color: rgba(var(--mdui-color-secondary-container), 0.2);
  padding: 1rem;
  width: calc(100% + 1rem);
  transform: translateX(0.5rem);
  margin: 0 -1rem;
  line-height: 1.8;
  backdrop-filter: blur(2px) !important;
}
.label-large-content img {
  height: 2rem;
  transform: translateY(0.5rem);
  border-radius: var(--mdui-shape-corner-small);
}
.label-large-content sub,
.label-large-content sub > * {
  vertical-align: middle;
}
.label-large-content mdui-button {
  vertical-align: middle;
  transform: translateY(-0.15rem);
  margin: 0 0.15rem;
}
.label {
  display: flex;
  align-items: center;
}
</style>
