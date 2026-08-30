<script setup lang="ts">
import '@mdui/icons/arrow-forward';
import '@mdui/icons/file-present.js';
import { MinecraftInstance } from '../../libs/instance';

const props = defineProps<MinecraftInstance>()
</script>

<template>
    <mdui-list-item class="card">
        <mdui-avatar src="/src/assets/mc_icon.png"
            slot="icon" class="icon" />
        <div class="text">
            <p class="main-text"><b>{{ props.name }}</b></p>
            <sub><mdui-icon-file-present class="text-icon file-present"></mdui-icon-file-present> {{ props.versionId }}</sub>
        </div>
        <mdui-icon-arrow-forward slot="end-icon" class="end-icon"></mdui-icon-arrow-forward>
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

.icon {
    border-radius: 0;
    background-color: rgba(0, 0, 0, 0);
}

.text {
    display: block;
    line-height: 0;
    margin-top: -0.8rem;
    p {
        color: rgb(var(--mdui-color-on-surface-variant))
    }
    sub {
        color: rgb(var(--mdui-color-on-surface-variant))
    }
}

.text-icon {
    font-size: small;
}

.file-present {
    transform: translateY(0.05rem);
}

.main-text {
    font-size: large;
}
</style>