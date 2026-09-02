import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { reactive, ref } from 'vue';
import { useToastStore } from './toast';
const { pushToast } = useToastStore();

export interface RunningInstance {
    id: string;
    name: string;
    versionId: string;
    path: string;
    icon?: string | null;
}

/**
 * 会话生命周期：
 * launching 启动中 -> running 运行中 -> exited 已停止 / error 启动失败
 * 进入 terminal 状态（exited/error）后不自动销毁，由用户在面板里手动销毁。
 */
export type RunningStatus = 'launching' | 'running' | 'exited' | 'error';

/** 日志来源：system 为启动器自身流程（natives/命令等），game 为 Minecraft 输出 */
export type LogKind = 'system' | 'game';

export interface IslandLogLine {
    text: string;
    kind: LogKind;
}

export interface RunningSession {
    id: number;
    instance: RunningInstance;
    summary: string;
    status: RunningStatus;
    launchedAt: number;
    /** 最近一次成为“活跃/展开目标”的时间戳，用于 Dock 排序 */
    lastActive: number;
    /** 由实例列表启动的新会话（非 Dock 点击），用于“直接展开”分支 */
    freshOpen: boolean;
    pid: number | null;
    exitCode: number | null;
    message?: string | null;
}

interface SessionStatePayload {
    sessionId: number;
    status: RunningStatus;
    pid?: number | null;
    exitCode?: number | null;
    message?: string | null;
}

interface SessionLogPayload {
    sessionId: number;
    text: string;
    kind: string;
}

interface LaunchReply {
    sessionId: number;
}

const MAX_LOG_LINES = 800;

let next_session_id = 1;

const sessions = ref<RunningSession[]>([]);
const expandedId = ref<number | null>(null);
/** Toast 避让所需的新 bottom 值（px）；0 表示不需要避让 */
const dockClearance = ref(0);
/** 每个会话的日志行（独立于会话是否已注册，事件先到会先缓冲） */
const logsBySid = reactive<Record<number, IslandLogLine[]>>({});
/** 最近一次状态（会话尚未创建时先记录，创建时补挂） */
const latestState = reactive<Record<number, Omit<SessionStatePayload, 'sessionId'>>>({});

function sort_by_active(list: RunningSession[]): RunningSession[] {
    return [...list].sort((a, b) => b.lastActive - a.lastActive);
}

/** 按最近活跃重排，让最新活跃/新启动的会话置顶（最近用过的排第一） */
export function reorder_recent() {
    sessions.value = sort_by_active(sessions.value);
}

/**
 * 注册会话（launch 成功拿到 sessionId 后调用）。
 * 会话可能在事件之后才创建，因此补挂已缓冲的日志与最新状态。
 */
export function launch_instance(instance: RunningInstance, sessionId?: number): RunningSession {
    const id = sessionId ?? next_session_id++;
    const st = latestState[id];
    const session: RunningSession = {
        id,
        instance,
        summary: instance.name,
        status: st?.status ?? 'launching',
        launchedAt: Date.now(),
        lastActive: Date.now(),
        freshOpen: true,
        pid: st?.pid ?? null,
        exitCode: st?.exitCode ?? null,
        message: st?.message ?? null,
    };
    if (!logsBySid[id]) logsBySid[id] = [];
    sessions.value = [session, ...sessions.value];
    expandedId.value = id;
    pushToast({ level: 'info', title: '正在启动 '+instance.name})
    return session;
}

/** 真实启动：调用后端创建会话，再用返回的 sessionId 注册岛 */
export async function launch_backend(instance: RunningInstance): Promise<number> {
    const reply = await invoke<LaunchReply>('launch_minecraft', {
        versionId: instance.versionId,
    });
    launch_instance(instance, reply.sessionId);
    return reply.sessionId;
}

/** 终止运行中的 Minecraft（保留面板，等待 exited 事件把状态置为“已停止”） */
export async function stop_session(id: number) {
    try {
        await invoke('stop_minecraft_session', { sessionId: id });
    } catch (e) {
        console.error('停止会话失败:', e);
        pushToast({ level: 'error', title: '停止实例 '+id+' 失败', message: (e as string).slice(0, 20) });
    }
}

/** 销毁面板：移除会话、日志与缓冲状态（仅应在 terminal 状态下由用户触发） */
export function destroy_session(id: number) {
    if (expandedId.value === id) expandedId.value = null;
    sessions.value = sessions.value.filter((s) => s.id !== id);
    delete logsBySid[id];
    delete latestState[id];
}

/** 点击 Dock 胶囊展开/切换：更新活跃时间并立即置顶（展开前先排位） */
export function focus_session(id: number) {
    const target = sessions.value.find((s) => s.id === id);
    if (!target) return;
    target.lastActive = Date.now();
    reorder_recent();
    expandedId.value = id;
}

export function collapse() {
    expandedId.value = null;
}

export function append_log(session_id: number, line: IslandLogLine) {
    if (!logsBySid[session_id]) logsBySid[session_id] = [];
    const arr = logsBySid[session_id];
    arr.push(line);
    if (arr.length > MAX_LOG_LINES) {
        arr.splice(0, arr.length - MAX_LOG_LINES);
    }
}

function patch_session_state(
    id: number,
    partial: Partial<Pick<RunningSession, 'status' | 'pid' | 'exitCode' | 'message'>>,
) {
    latestState[id] = {
        status: partial.status ?? latestState[id]?.status ?? 'launching',
        pid: partial.pid ?? null,
        exitCode: partial.exitCode ?? null,
        message: partial.message ?? null,
    };
    const s = sessions.value.find((x) => x.id === id);
    if (!s) return;
    if (partial.status !== undefined) s.status = partial.status;
    if (partial.pid !== undefined) s.pid = partial.pid;
    if (partial.exitCode !== undefined) s.exitCode = partial.exitCode;
    if (partial.message !== undefined) s.message = partial.message;
}

function apply_state(payload: SessionStatePayload) {
    const { sessionId, ...rest } = payload;
    latestState[sessionId] = rest;
    const s = sessions.value.find((x) => x.id === sessionId);
    if (!s) return;
    s.status = payload.status;
    s.pid = payload.pid ?? null;
    s.exitCode = payload.exitCode ?? null;
    if (payload.message !== undefined) s.message = payload.message;
}

/** 纯前端预览（无 Tauri 后端）时的演示会话：模拟启动 → 运行并滚动几条假日志 */
export function demo_session(instance: RunningInstance): number {
    const s = launch_instance(instance);
    append_log(s.id, {
        text: `[系统] 未连接启动后端，进入演示模式（${instance.versionId}）`,
        kind: 'system',
    });
    setTimeout(() => {
        patch_session_state(s.id, { status: 'running', pid: 10000 + s.id * 137 });
    }, 1200);
    setTimeout(() => {
        append_log(s.id, { text: '[main/INFO]: Minecraft 正在初始化…', kind: 'game' });
    }, 1400);
    setTimeout(() => {
        append_log(s.id, { text: '[Render thread/INFO]: 已创建 GLFW 窗口，帧率 120', kind: 'game' });
    }, 2200);
    return s.id;
}

function handle_log(payload: SessionLogPayload) {
    append_log(payload.sessionId, {
        text: payload.text,
        kind: payload.kind === 'system' ? 'system' : 'game',
    });
}

let unlisteners: UnlistenFn[] = [];
let events_initialized = false;

/** 订阅后端会话事件（由全局 Island 宿主调用一次即可） */
export async function init_session_events() {
    if (events_initialized) return;
    events_initialized = true;
    const state = await listen<SessionStatePayload>('mc-session-state', (e) => {
        apply_state(e.payload);
    });
    const log = await listen<SessionLogPayload>('mc-session-log', (e) => {
        handle_log(e.payload);
    });
    unlisteners = [state, log];
}

export function dispose_session_events() {
    for (const un of unlisteners) un();
    unlisteners = [];
    events_initialized = false;
}

export function useRunningStore() {
    return {
        sessions,
        expandedId,
        dockClearance,
        logsBySid,
        latestState,
        launch_instance,
        launch_backend,
        demo_session,
        focus_session,
        reorder_recent,
        collapse,
        stop_session,
        destroy_session,
        append_log,
    };
}
