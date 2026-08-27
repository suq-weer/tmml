<script lang="ts" setup>
import '@mdui/icons/arrow-forward.js';
import { ALL_PHASES, DownloadPhaseState, DownloadToastState } from '../libs/toast';

defineProps<{ state: DownloadToastState }>();

function phase_percent(ps: DownloadPhaseState): number {
    if (ps.totalFiles === 0) return 0;
    return Math.min(ps.doneFiles / ps.totalFiles, 1);
}

function basename(path: string): string {
    const idx = path.lastIndexOf('/');
    return idx >= 0 ? path.slice(idx + 1) : path;
}
</script>

<template>
    <div class="detail">
        <div v-for="p in ALL_PHASES" :key="p" class="phase-row">
            <mdui-circular-progress
                v-if="state.phases[p].totalFiles > 0"
                class="phase-spinner"
                :value="phase_percent(state.phases[p])"
            ></mdui-circular-progress>
            <mdui-circular-progress v-else class="phase-spinner"></mdui-circular-progress>
            <span class="phase-label">下载 {{ p }} 阶段</span>
            <span v-if="state.phases[p].totalFiles > 0" class="phase-pct">
                {{ Math.round(phase_percent(state.phases[p]) * 100) }}%
            </span>
            <span v-if="state.phases[p].reusedFiles > 0" class="phase-reused">
                已复用 {{ state.phases[p].reusedFiles }} 个
            </span>

            <ul v-if="state.phases[p].files.length > 0" class="file-list">
                <li v-for="f in state.phases[p].files" :key="f.name" class="file-row">
                    <mdui-icon-arrow-forward class="file-arrow"></mdui-icon-arrow-forward>
                    <span class="file-pct">{{ Math.round(f.percent) }}%</span>
                    <span class="file-name" :title="f.name">{{ basename(f.name) }}</span>
                </li>
            </ul>
        </div>
        <p v-if="state.message" class="detail-message">{{ state.message }}</p>
    </div>
</template>

<style scoped>
.detail {
    margin-top: 4px;
}

.phase-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin: 6px 0;
}

.phase-spinner {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
}

.phase-label {
    font-size: 14px;
    color: rgb(var(--mdui-color-on-surface-variant));
}

.phase-pct {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: rgb(var(--mdui-color-on-surface-variant));
}

.phase-reused {
    font-size: 12px;
    color: rgb(var(--mdui-color-primary));
}

.file-list {
    list-style: none;
    margin: 2px 0 0 24px;
    padding: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.file-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: rgb(var(--mdui-color-on-surface-variant));
}

.file-arrow {
    font-size: 14px;
    flex-shrink: 0;
}

.file-pct {
    min-width: 34px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
}

.file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.detail-message {
    margin: 4px 0 0 0;
    color: rgb(var(--mdui-color-error));
    word-break: break-all;
}
</style>
