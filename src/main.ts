import { createApp } from "vue";
import App from "./App.vue";
import { invoke } from '@tauri-apps/api/core';
import 'mdui/mdui.css';
import 'mdui';
import { setColorScheme } from "mdui";
import { useDark } from "@vueuse/core";
import { debug_f, warn_f } from "./libs/query_backend.ts";

// 获取系统主题色
invoke<String>('get_system_color')
    .then((color) => {
        setColorScheme(color.valueOf())
        debug_f("已成功应用系统强调色至主题界面！")
    })
    .catch((e) => {
        warn_f("获取系统强调色失败: " + String(e))
        warn_f("尝试应用默认强调色……")
        setColorScheme("#4A92CB")
    });
// 自动深色切换
const mode = useDark({
    selector: 'html',
    attribute: 'class',
    valueDark: 'mdui-theme-dark',
    valueLight: 'mdui-theme-light',
})
debug_f("系统深色模式: " + mode.value)

createApp(App).mount("#app");
