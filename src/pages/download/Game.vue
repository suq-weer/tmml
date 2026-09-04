<script setup lang="ts">
import VersionListCard from "../../components/cards/VersionListCard.vue";
</script>

<template>
  <VersionListCard version_mode="RELEASE" />
  <VersionListCard version_mode="SNAPSHOT" />
  <VersionListCard version_mode="FOOL" />
</template>

<style lang="css" scoped></style>
