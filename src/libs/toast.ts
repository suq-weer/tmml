import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';

export type ToastLevel = 'info' | 'success' | 'warning' | 'error';

export type DownloadPhase =
    | 'versionJson'
    | 'clientJar'
    | 'libraries'
    | 'assetsIndex'
    | 'logging'
    | 'assets';

export const ALL_PHASES: DownloadPhase[] = [
    'versionJson',
    'clientJar',
    'libraries',
    'assetsIndex',
    'logging',
    'assets',
];

export interface ToastPayload {
    level: ToastLevel;
    title: string;
    message?: string;
    kind?: string;
    versionId?: string;
}

export interface FileProgress {
    name: string;
    bytesDone: number;
    size: number;
    percent: number;
}

export interface DownloadPhaseState {
    phase: DownloadPhase;
    doneFiles: number;
    totalFiles: number;
    finished: boolean;
    reusedFiles: number;
    files: FileProgress[];
}

export interface DownloadToastState {
    versionId: string;
    level: ToastLevel;
    title: string;
    message?: string;
    finished: boolean;
    phase: DownloadPhase;
    bytesDone: number;
    bytesTotal: number;
    speed: number;
    index: number;
    count: number;
    phases: Record<DownloadPhase, DownloadPhaseState>;
}

export interface ToastItem extends ToastPayload {
    id: number;
    timestamp: number;
    download?: DownloadToastState;
}

export interface DownloadProgressPayload {
    versionId: string;
    phase: DownloadPhase;
    name: string;
    index: number;
    count: number;
    bytesDone: number;
    bytesTotal: number;
    speed: number;
    finished: boolean;
    fileBytesDone: number;
    fileSize: number;
    reused: boolean;
    reusedCount: number;
}

const AUTO_DISMISS_MS: Record<ToastLevel, number> = {
    info: 5000,
    success: 5000,
    warning: 5000,
    error: 8000,
};

const MAX_FILES_PER_PHASE = 20;

let next_id = 1;

const toasts = ref<ToastItem[]>([]);
const notifications = ref<ToastItem[]>([]);

function empty_phase(phase: DownloadPhase): DownloadPhaseState {
    return { phase, doneFiles: 0, totalFiles: 0, finished: false, reusedFiles: 0, files: [] };
}

function create_download_state(payload: ToastPayload): DownloadToastState {
    const phases = {} as Record<DownloadPhase, DownloadPhaseState>;
    for (const p of ALL_PHASES) phases[p] = empty_phase(p);
    return {
        versionId: payload.versionId ?? '',
        level: payload.level,
        title: payload.title,
        message: payload.message,
        finished: false,
        phase: 'versionJson',
        bytesDone: 0,
        bytesTotal: 0,
        speed: 0,
        index: 0,
        count: 0,
        phases,
    };
}

function find_download(versionId?: string) {
    return toasts.value.find((t) => t.kind === 'download' && t.versionId === versionId)
        ?? notifications.value.find((n) => n.kind === 'download' && n.versionId === versionId);
}

function schedule_dismiss(id: number, ms: number) {
    setTimeout(() => dismissToast(id), ms);
}

function upsert_download(payload: ToastPayload) {
    const existing = find_download(payload.versionId);
    if (existing && existing.download) {
        existing.level = payload.level;
        existing.title = payload.title;
        existing.message = payload.message;
        existing.download.level = payload.level;
        existing.download.title = payload.title;
        existing.download.message = payload.message;
        if (payload.level === 'success' || payload.level === 'error') {
            existing.download.finished = true;
            schedule_dismiss(existing.id, AUTO_DISMISS_MS[payload.level]);
        }
        return;
    }
    const item: ToastItem = {
        id: next_id++,
        level: payload.level,
        title: payload.title,
        message: payload.message,
        timestamp: Date.now(),
        kind: 'download',
        versionId: payload.versionId,
        download: create_download_state(payload),
    };
    toasts.value.push(item);
    notifications.value.unshift(item);
}

function pushToast(payload: ToastPayload) {
    if (payload.kind === 'download') {
        upsert_download(payload);
        return;
    }
    const item: ToastItem = {
        id: next_id++,
        level: payload.level,
        title: payload.title,
        message: payload.message,
        timestamp: Date.now(),
    };
    toasts.value.push(item);
    notifications.value.unshift(item);
    schedule_dismiss(item.id, AUTO_DISMISS_MS[payload.level]);
}

function applyProgress(p: DownloadProgressPayload) {
    const item = find_download(p.versionId);
    if (!item?.download) return;
    const d = item.download;
    d.phase = p.phase;
    d.index = p.index;
    d.count = p.count;
    d.bytesDone = p.bytesDone;
    d.bytesTotal = p.bytesTotal;
    d.speed = p.speed;

    const ps = d.phases[p.phase];
    if (p.reusedCount !== undefined) {
        ps.reusedFiles = p.reusedCount;
    }
    if (p.finished) {
        ps.doneFiles = p.count;
        ps.totalFiles = p.count;
        ps.finished = true;
    } else {
        ps.totalFiles = p.count;
        ps.doneFiles = Math.max(ps.doneFiles, Math.min(p.index - 1, p.count));
        ps.finished = false;
    }

    if (p.name) {
        const percent = p.fileSize > 0 ? Math.min(100, (p.fileBytesDone / p.fileSize) * 100) : 0;
        if (p.reused || percent >= 100) {
            // 已复用/已完成下载的文件不参与前 20 展示
            ps.files = ps.files.filter((f) => f.name !== p.name);
            return;
        }
        const existing = ps.files.find((f) => f.name === p.name);
        if (existing) {
            existing.bytesDone = p.fileBytesDone;
            existing.size = p.fileSize;
            existing.percent = percent;
        } else {
            ps.files.push({ name: p.name, bytesDone: p.fileBytesDone, size: p.fileSize, percent });
        }
        ps.files.sort((a, b) => b.percent - a.percent);
        if (ps.files.length > MAX_FILES_PER_PHASE) {
            ps.files.length = MAX_FILES_PER_PHASE;
        }
    }
}

// 进度事件合并：短时间内的大量事件先缓冲，在下一帧统一应用，避免事件洪泛占满主线程
const pending_progress: DownloadProgressPayload[] = [];
let raf_scheduled = false;
let flush_timer: ReturnType<typeof setTimeout> | undefined;

function flushProgress() {
    if (!raf_scheduled) return;
    raf_scheduled = false;
    if (flush_timer !== undefined) {
        clearTimeout(flush_timer);
        flush_timer = undefined;
    }
    if (pending_progress.length === 0) return;
    const events = pending_progress.splice(0, pending_progress.length);
    for (const p of events) applyProgress(p);
}

function handleDownloadProgress(p: DownloadProgressPayload) {
    pending_progress.push(p);
    if (raf_scheduled) return;
    raf_scheduled = true;
    requestAnimationFrame(flushProgress);
    // 兜底：窗口不可见时 rAF 不触发，用定时器保证进度仍会应用
    flush_timer = setTimeout(flushProgress, 100);
}

function cancel_if_downloading(item: ToastItem | undefined) {
    if (item?.kind === 'download' && item.download && !item.download.finished) {
        invoke('cancel_minecraft_download', { versionId: item.versionId })
            .catch((e) => console.error('取消下载失败:', e));
    }
}

function dismissToast(id: number) {
    const item = toasts.value.find((t) => t.id === id);
    cancel_if_downloading(item);
    toasts.value = toasts.value.filter((t) => t.id !== id);
}

function removeNotification(id: number) {
    const item = notifications.value.find((n) => n.id === id);
    cancel_if_downloading(item);
    notifications.value = notifications.value.filter((n) => n.id !== id);
    toasts.value = toasts.value.filter((t) => t.id !== id);
}

function markAllRead() {
    notifications.value = [];
}

export function useToastStore() {
    return {
        toasts,
        notifications,
        pushToast,
        dismissToast,
        removeNotification,
        markAllRead,
        handleDownloadProgress,
    };
}
