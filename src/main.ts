import { createApp } from "vue";
import App from "./App.vue";
import { invoke } from "@tauri-apps/api/core";
import "./assets/main.css";
import "mdui/mdui.css";
import "mdui";
import "./assets/fonts/fonts.css";
import { setColorScheme } from "mdui";
import { useDark } from "@vueuse/core";
import { createMemoryHistory, createRouter } from "vue-router";
import { interceptConsole } from "@fltsci/tauri-plugin-tracing";
import { trackHistory } from "./libs/navigation";
import Notifications from "./pages/Notifications.vue";
import ProfileManagement from "./pages/ProfileManagement.vue";
import Home from "./pages/Home.vue";
import Download from "./pages/Download.vue";
import ComingSoon from "./pages/ComingSoon.vue";
import Game from "./pages/download/Game.vue";

// 初始化日志系统
interceptConsole({ preserveOriginal: true });

// 获取系统主题色
invoke<String>("get_system_color")
  .then((color) => {
    setColorScheme(color.valueOf());
    console.debug("已成功应用系统强调色至主题界面！");
  })
  .catch((e) => {
    console.warn("获取系统强调色失败: " + String(e));
    console.warn("尝试应用默认强调色……");
    setColorScheme("#4A92CB");
  });
// 自动深色切换
const mode = useDark({
  selector: "html",
  attribute: "class",
  valueDark: "mdui-theme-dark",
  valueLight: "mdui-theme-light",
});
console.debug("系统深色模式: " + mode.value);

// 路由创建
const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    { path: "/", component: Home },
    {
      path: "/download",
      component: Download,
      children: [
        { path: "", redirect: { name: "download-game" } },
        { path: "game", name: "download-game", component: Game },
        { path: "modpack", name: "download-modpack", component: ComingSoon },
        { path: "mod", name: "download-mod", component: ComingSoon },
        {
          path: "resourcepack",
          name: "download-resourcepack",
          component: ComingSoon,
        },
        { path: "shader", name: "download-shader", component: ComingSoon },
        { path: "map", name: "download-map", component: ComingSoon },
      ],
    },
    { path: "/notifications", component: Notifications },
    { path: "/profiles", component: ProfileManagement },
  ],
});

trackHistory(router);

createApp(App).use(router).mount("#app");
