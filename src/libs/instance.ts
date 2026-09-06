import { invoke } from '@tauri-apps/api/core';

export interface InstanceConfig {
    launchCommandPrefix: string[];
    launchCommandSuffix: string[];
    javaPath?: string | null;
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

/** 新建实例：下载指定版本并建立实例的基本内容。实例以名称（目录名）为唯一标识，同一版本可多次安装、多实例共存 */
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

/** 读取单个实例完整信息。`instanceId` 为实例标识（＝实例目录名） */
export function get_instance(instanceId: string) {
    return invoke<InstanceInfo | null>('get_instance', { instanceId });
}

/** 更新实例（`instanceId` 为实例标识＝目录名） */
export function update_instance(instanceId: string, name?: string, config?: InstanceConfig) {
    return invoke<InstanceInfo>('update_instance', { instanceId, name, config });
}
