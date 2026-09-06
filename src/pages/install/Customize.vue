<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import SchemaFormCard from "../../components/cards/SchemaFormCard.vue";
import StepActions from "./StepActions.vue";
import { single_version_from_query } from "../../libs/install_wizard";
import {
  INSTANCE_CUSTOMIZE_SCHEMA,
  type InstanceCustomizeValues,
  get_customize_values,
  set_customize_values,
  set_instance_name_context,
} from "../../libs/instance_customize";
import { list_instances } from "../../libs/instance";
import { useToastStore } from "../../libs/toast";

const route = useRoute();
const { pushToast } = useToastStore();

const version = computed(() => single_version_from_query(route.query));
const version_id = computed(() => String(version.value?.id ?? ""));
const cardRef = ref<InstanceType<typeof SchemaFormCard> | null>(null);

const customize = ref<InstanceCustomizeValues>(
  get_customize_values(version_id.value),
);

/** 加载本地实例列表作为查重上下文；列表就绪后重新校验名称字段（可能命中“同名/默认版本已存在”） */
let nameContextReady: Promise<void> = Promise.resolve();

function loadNameContext(): Promise<void> {
  if (!version_id.value) return Promise.resolve();
  return (async () => {
    try {
      const instances = await list_instances();
      set_instance_name_context(version_id.value, instances);
      await nextTick();
      // 校验整表：会把「实例名称」字段的查重/合法性问题直接回显在输入框下方
      cardRef.value?.validate();
    } catch (e) {
      console.error("读取本地实例列表失败，跳过名称查重:", e);
    }
  })();
}

onMounted(() => {
  nameContextReady = loadNameContext();
});

function onUpdate(values: Record<string, unknown>): void {
  const typed = values as InstanceCustomizeValues;
  customize.value = typed;
  set_customize_values(typed);
}

async function beforeNext(): Promise<boolean> {
  // 查重依赖本地实例列表，务必等它加载完成后再放行，避免“默认名称撞已装实例”被跳过
  await nameContextReady;
  const card = cardRef.value;
  if (!card) return true;
  if (card.validate()) return true;
  pushToast({
    level: "warning",
    title: "表单校验未通过",
    message: card.firstError() ?? "请修正标注的字段后再继续",
  });
  return false;
}
</script>

<template>
  <div class="step-page">
    <section v-if="!version" class="empty-state">
      <p class="empty-title">尚未选择要安装的 Minecraft 版本</p>
      <sub class="empty-desc">
        请先在下载页选择一个游戏版本，再回到本向导继续
      </sub>
    </section>

    <template v-else>
      <SchemaFormCard
        v-if="version"
        ref="cardRef"
        :schema="INSTANCE_CUSTOMIZE_SCHEMA"
        title="个性化设置"
        subtitle="留空的参数将以全局默认配置托底"
        :model-value="customize"
        @update:model-value="onUpdate"
      />
    </template>

    <StepActions :allow-next="!!version" :before-next="beforeNext" />
  </div>
</template>

<style scoped>
.step-page {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem;
  border: 1px dashed rgb(var(--mdui-color-outline-variant));
  border-radius: var(--mdui-shape-corner-extra-large);
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
</style>
