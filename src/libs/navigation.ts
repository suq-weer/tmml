import type { Router, RouterHistory } from "vue-router";

let routerRef: Router | null = null;
let mirror: string[] = [];
let position = 0;
let tracked = false;
let rawGo: RouterHistory["go"] | null = null;

function normalize(path: string): string {
  return path === "" ? "/" : path;
}

function mirrorPush(path: string): void {
  position += 1;
  if (position !== mirror.length) mirror.splice(position);
  mirror.push(normalize(path));
}

function mirrorReplace(path: string): void {
  mirror.splice(position, 1);
  position -= 1;
  mirrorPush(path);
}

function mirrorGo(delta: number): void {
  position = Math.max(0, Math.min(position + delta, mirror.length - 1));
}

export function trackHistory(router: Router): void {
  if (tracked) return;
  tracked = true;
  routerRef = router;
  const history = router.options.history as RouterHistory;
  mirror = [normalize(history.location)];
  position = 0;

  const push = history.push.bind(history);
  const replace = history.replace.bind(history);
  const go = history.go.bind(history);
  rawGo = go;

  history.push = (to) => {
    push(to);
    mirrorPush(to);
  };
  history.replace = (to) => {
    replace(to);
    mirrorReplace(to);
  };
  history.go = (delta, triggerListeners) => {
    go(delta, triggerListeners);
    mirrorGo(delta);
  };
}

export function leaveCurrentSection(prefix: string): void {
  const router = routerRef;
  if (!router || mirror.length === 0) return;
  if (!normalize(mirror[position]).startsWith(prefix)) {
    router.back();
    return;
  }
  let steps = 0;
  for (let i = position; i >= 0; i--) {
    if (normalize(mirror[i]).startsWith(prefix)) steps += 1;
    else break;
  }
  if (steps > 0) router.go(-steps);
}

/**
 * 把整段连续的路由（如 /install 下的所有步骤）从历史栈中摘除，
 * 再导航到目标路由。此后 router.back() 会直接回到进入该段路由之前的页面。
 *
 * 做法：先把 memory history 的游标无声地退回到段首前一个条目，
 * 再让 router.push(to) 完成“截断段内条目 + 追加 to”的入栈。
 */
export function collapseSectionAndPush(prefix: string, to: string): void {
  const router = routerRef;
  if (!router || !rawGo || mirror.length === 0) {
    router?.push(to);
    return;
  }
  if (!normalize(mirror[position]).startsWith(prefix)) {
    router.push(to);
    return;
  }
  let steps = 0;
  for (let i = position; i >= 0; i--) {
    if (normalize(mirror[i]).startsWith(prefix)) steps += 1;
    else break;
  }
  if (steps === 0) {
    router.push(to);
    return;
  }
  rawGo(-steps, false);
  position -= steps;
  router.push(to);
}

/**
 * 在导航守卫里调用：当前位置若位于以 prefix 开头的一段连续路由上，
 * 则把 memory history 游标无声地退回该段之前，随后由本次导航的 push 自然截断并覆盖该段。
 * 适合「任务已提交页被任意入口（按钮 / AppBar 等）跳走时，摘除整个 /install 历史块」的场景。
 */
export function rewindSectionCursor(prefix: string): void {
  if (!rawGo || mirror.length === 0) return;
  let steps = 0;
  for (let i = position; i >= 0; i--) {
    if (normalize(mirror[i]).startsWith(prefix)) steps += 1;
    else break;
  }
  if (steps === 0) return;
  rawGo(-steps, false);
  position -= steps;
}
