import type { Router, RouterHistory } from "vue-router";

let routerRef: Router | null = null;
let mirror: string[] = [];
let position = 0;
let tracked = false;

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
