import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';

export type AuthType = 'offline' | 'microsoft' | 'authlib-injector';

export interface GameProfile {
    id: string;
    name: string;
    authType: AuthType;
    username?: string | null;
    authData?: unknown;
    createdAt: string;
}

export interface LastLaunchedInstance {
    versionId: string;
    name: string;
    dir: string;
}

const current = ref<GameProfile | null>(null);
const lastLaunched = ref<LastLaunchedInstance | null>(null);
const profiles = ref<GameProfile[]>([]);

export async function refresh() {
    profiles.value = await list_game_profiles();
    current.value = await get_current_profile();
    lastLaunched.value = await get_last_launched_instance();
}

export function list_game_profiles() {
    return invoke<GameProfile[]>('list_game_profiles');
}

export function get_current_profile() {
    return invoke<GameProfile | null>('get_current_profile');
}

export function create_game_profile(authType: AuthType, username?: string) {
    return invoke<GameProfile>('create_game_profile', { authType, username });
}

export function delete_game_profile(id: string) {
    return invoke<void>('delete_game_profile', { id });
}

export function set_default_profile(profileId: string | null) {
    return invoke<void>('set_default_profile', { profileId });
}

export function get_last_launched_instance() {
    return invoke<LastLaunchedInstance | null>('get_last_launched_instance');
}

export function get_instance_icon(dir: string) {
    return invoke<string | null>('get_instance_icon', { dirName: dir });
}

export function useProfileStore() {
    return { current, lastLaunched, profiles, refresh };
}
