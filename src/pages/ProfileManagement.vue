<script lang="ts" setup>
import "@mdui/icons/person-add-alt-1.js";
import "@mdui/icons/delete--outlined.js";
import "@mdui/icons/arrow-back.js";
import { onMounted, ref } from "vue";
import {
  create_game_profile,
  delete_game_profile,
  set_default_profile,
  useProfileStore,
} from "../libs/profile";
import { useToastStore } from "../libs/toast";

const { profiles, current, refresh } = useProfileStore();
const { pushToast } = useToastStore();

const username = ref("");

onMounted(() => {
  refresh();
});

function is_current(id: string) {
  return current.value?.id === id;
}

async function create_offline() {
  const name = username.value.trim();
  if (!name) {
    pushToast({ level: "warning", title: "请输入玩家名" });
    return;
  }
  try {
    await create_game_profile("offline", name);
    username.value = "";
    await refresh();
    pushToast({ level: "success", title: "已创建离线档案", message: name });
  } catch (e) {
    pushToast({ level: "error", title: "创建失败", message: String(e) });
  }
}

async function switch_to(id: string) {
  try {
    await set_default_profile(id);
    await refresh();
    pushToast({ level: "success", title: "已切换档案" });
  } catch (e) {
    pushToast({ level: "error", title: "切换失败", message: String(e) });
  }
}

async function remove(id: string) {
  try {
    await delete_game_profile(id);
    await refresh();
    pushToast({ level: "success", title: "已删除档案" });
  } catch (e) {
    pushToast({ level: "error", title: "删除失败", message: String(e) });
  }
}
</script>

<template>
  <div class="profiles-page">
    <header class="page-head">
      <div class="head-row">
        <mdui-button-icon class="arrow-back-button" @click="$router.back()">
          <mdui-icon-arrow-back></mdui-icon-arrow-back>
        </mdui-button-icon>
        <h2>游戏档案管理</h2>
      </div>
      <p class="page-desc">
        管理准备在游戏里启动的玩家账号。当前支持离线登录，其它登录方式开发中。
      </p>
    </header>

    <mdui-card variant="outlined" class="card">
      <div class="card-title">新建离线档案</div>
      <div class="create-row">
        <mdui-text-field
          label="玩家名"
          :value="username"
          @input="username = ($event.target as any).value"
        ></mdui-text-field>
        <mdui-button @click="create_offline()">
          <mdui-icon-person-add-alt-1 slot="icon"></mdui-icon-person-add-alt-1>
          新建
        </mdui-button>
      </div>
    </mdui-card>

    <mdui-card variant="outlined" class="card">
      <div class="card-title">档案列表</div>
      <template v-if="profiles.length > 0">
        <mdui-divider></mdui-divider>
        <div v-for="p in profiles" :key="p.id" class="profile-row">
          <div class="row-main">
            <span class="row-name">{{ p.name }}</span>
            <mdui-badge>{{ p.authType }}</mdui-badge>
            <mdui-badge v-if="is_current(p.id)" class="current-badge"
              >当前</mdui-badge
            >
          </div>
          <div class="row-actions">
            <mdui-button variant="tonal" @click="switch_to(p.id)"
              >切换</mdui-button
            >
            <mdui-button-icon @click="remove(p.id)">
              <mdui-icon-delete--outlined></mdui-icon-delete--outlined>
            </mdui-button-icon>
          </div>
        </div>
      </template>
      <div v-else class="empty">暂无档案，请先新建一个离线档案</div>
    </mdui-card>
  </div>
</template>

<style scoped>
.profiles-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 820px;
  margin: 0 auto;
  padding: 20px 16px 40px;
}

.head-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.arrow-back-button {
  transform: translateY(-2px);
}

.page-head h2 {
  margin: 0 0 4px;
}

.page-desc {
  margin: 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}

.card {
  padding: 16px;
}

.card-title {
  font-weight: 600;
  margin-bottom: 12px;
}

.create-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.create-row mdui-text-field {
  flex: 1;
  min-width: 200px;
}

.profile-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 0;
}

.profile-row + .profile-row {
  border-top: 1px solid rgb(var(--mdui-color-outline-variant));
}

.row-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.row-name {
  font-weight: 600;
}

.current-badge {
  background-color: rgb(var(--mdui-color-primary-container));
  color: rgb(var(--mdui-color-on-primary-container));
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.empty {
  text-align: center;
  padding: 32px 0;
  color: rgb(var(--mdui-color-on-surface-variant));
}
</style>
