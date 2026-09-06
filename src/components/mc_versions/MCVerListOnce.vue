<script setup lang="ts">
import "@mdui/icons/arrow-forward";
import "@mdui/icons/alarm-add";
import "@mdui/icons/commit";
import { SingleVersion } from "../../libs/mc_version";
import { computed } from "vue";
import { useRouter } from "vue-router";
import { single_version_to_query } from "../../libs/install_wizard";

const props = withDefaults(
  defineProps<SingleVersion & { interactive?: boolean }>(),
  { interactive: true },
);
const router = useRouter();

function goInstall(): void {
  if (props.interactive === false) return;
  router.push({ path: "/install/env", query: single_version_to_query(props) });
}

function fmt_local_time(iso: String): string {
  const d = new Date(iso as string);
  if (Number.isNaN(d.getTime())) {
    return iso as string;
  }
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

const timeText = computed(() => fmt_local_time(props.releaseTime));
</script>

<template>
  <mdui-list-item
    class="card"
    :class="{ 'card-static': props.interactive === false }"
    :clickable="props.interactive !== false"
    @click="goInstall"
  >
    <mdui-avatar
      :src="
        props.type == 'snapshot'
          ? '/src/assets/mc_icon_beta.png'
          : '/src/assets/mc_icon.png'
      "
      slot="icon"
      class="icon"
    />
    <div class="text">
      <p class="main-text">
        <b>{{ props.id }}</b>
      </p>
      <sub>
        <mdui-icon-alarm-add class="text-icon clock"></mdui-icon-alarm-add>
        {{ timeText }} |
        <mdui-icon-commit class="text-icon commit"></mdui-icon-commit>
        {{ props.type == "snapshot" ? "快照" : "正式版" }}</sub
      >
    </div>
    <mdui-icon-arrow-forward
      slot="end-icon"
      class="end-icon"
    ></mdui-icon-arrow-forward>
  </mdui-list-item>
</template>

<style lang="css">
.card {
  .end-icon {
    display: none;
  }
}

.card:hover {
  .end-icon {
    display: block;
  }
}

.card-static,
.card-static:hover {
  cursor: default;
  .end-icon {
    display: none;
  }
}

.icon {
  border-radius: 0;
  background-color: rgba(0, 0, 0, 0);
}

.text {
  display: block;
  line-height: 0;
  margin-top: -0.8rem;
  p {
    color: rgb(var(--mdui-color-on-surface-variant));
  }
  sub {
    color: rgb(var(--mdui-color-on-surface-variant));
  }
}

.text-icon {
  font-size: small;
}

.clock {
  transform: translateY(0.05rem);
}

.commit {
  transform: translateY(0.05rem);
}

.main-text {
  font-size: large;
}
</style>
