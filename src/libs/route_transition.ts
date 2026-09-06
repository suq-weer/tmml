import { ref } from "vue";
import type { Router } from "vue-router";

export type RouteTransitionName = "push" | "pop";

const transitionName = ref<RouteTransitionName>("push");

let tracked = false;

/** 让顶层与所有嵌套 RouterView 共享同一切换动画的方向 */
export function trackRouteTransition(router: Router): void {
  if (tracked) return;
  tracked = true;

  let direction: RouteTransitionName = "push";
  router.options.history.listen((_to, _from, info) => {
    direction = info.delta < 0 ? "pop" : "push";
  });
  router.afterEach(() => {
    transitionName.value = direction;
    direction = "push";
  });
}

export function useRouteTransitionName() {
  return transitionName;
}
