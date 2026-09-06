<script setup lang="ts">
import VersionListCard from "../../components/cards/VersionListCard.vue";
</script>

<template>
  <div class="game-versions">
    <VersionListCard version_mode="RELEASE" />
    <VersionListCard version_mode="SNAPSHOT" />
    <VersionListCard version_mode="FOOL" />
  </div>
</template>

<style scoped>
.game-versions {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
</style>
