import { invoke } from '@tauri-apps/api/core';

export interface InstanceConfig {
    launchCommandPrefix: string[];
    launchCommandSuffix: string[];
    jvmArgs: string[];
    gameArgs: string[];
    width?: number;
    height?: number;
}

export interface InstanceInfo {
    id: string;
    versionId: string;
    name: string;
    path: string;
    createdAt: string;
    config: InstanceConfig;
}

export interface MinecraftInstance {
    id: string;
    versionId: string;
    name: string;
    path: string;
    createdAt: string;
}

/** 新建实例：下载指定版本并建立实例的基本内容 */
export function create_instance(versionId: string, instanceName?: string, config?: InstanceConfig) {
    return invoke<void>('download_minecraft_version', {
        versionId,
        instanceName: instanceName,
        instanceConfig: config,
    });
}

export function list_instances() {
    return invoke<MinecraftInstance[]>('list_instances');
}

export function get_instance(versionId: string) {
    return invoke<InstanceInfo | null>('get_instance', { versionId });
}

export function update_instance(versionId: string, name?: string, config?: InstanceConfig) {
    return invoke<InstanceInfo>('update_instance', { versionId, name, config });
}
