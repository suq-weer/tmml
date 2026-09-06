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
import { trackHistory, rewindSectionCursor } from "./libs/navigation";
import { INSTALL_SUBMITTED_PATH } from "./libs/install_wizard";
import Notifications from "./pages/Notifications.vue";
import ProfileManagement from "./pages/ProfileManagement.vue";
import Home from "./pages/Home.vue";
import Download from "./pages/Download.vue";
import ComingSoon from "./pages/ComingSoon.vue";
import Game from "./pages/download/Game.vue";
import Install from "./pages/Install.vue";
import InstallEnv from "./pages/install/Env.vue";
import InstallCustomize from "./pages/install/Customize.vue";
import InstallStart from "./pages/install/Start.vue";
import InstallSubmitted from "./pages/install/Submitted.vue";

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
    {
      path: "/install",
      component: Install,
      children: [
        { path: "", redirect: { name: "install-env" } },
        { path: "env", name: "install-env", component: InstallEnv },
        {
          path: "customize",
          name: "install-customize",
          component: InstallCustomize,
        },
        {
          path: "download",
          name: "install-download",
          component: InstallStart,
        },
        {
          path: "submitted",
          name: "install-submitted",
          component: InstallSubmitted,
        },
      ],
    },
  ],
});

trackHistory(router);

// 离开向导“任务已提交”页（按钮 / AppBar 等任意入口）时，把整段 /install 历史摘除，
// 让 router.back() 直接回到进入向导前的页面，而不是回到向导内部。
router.beforeEach((to, from) => {
  if (from.path === INSTALL_SUBMITTED_PATH && !to.path.startsWith("/install")) {
    rewindSectionCursor("/install");
  }
  return true;
});

createApp(App).use(router).mount("#app");
