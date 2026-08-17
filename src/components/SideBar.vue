<script lang="ts">
import '@mdui/icons/cloud--outlined.js';
</script>

<template>
    <mdui-navigation-rail value="recent">
        <mdui-navigation-rail-item value="recent"><mdui-icon-cloud--outlined slot="icon"></mdui-icon-cloud--outlined>Recent</mdui-navigation-rail-item>
        <mdui-navigation-rail-item icon="image--outlined" value="images">Images</mdui-navigation-rail-item>
        <mdui-navigation-rail-item icon="library_music--outlined" value="library">Library</mdui-navigation-rail-item>
    </mdui-navigation-rail>
</template>