// 把 MC/启动器日志文本转成可安全 v-html 的片段：
// - 游戏输出：解析 ANSI SGR 转义（真彩色/16色/加粗），无转义时按日志级别补色；
// - 启动器 system 输出：按前缀上色。
// 所有文本一律先 HTML 转义，杜绝注入。

import type { LogKind } from './running';

// xterm 16 色（gruvbox 风格，兼顾深浅主题可读性）
const PALETTE = [
    '#1b1b1b', '#cc241d', '#98971a', '#d79921', '#458588', '#b16286', '#689d6a', '#a89984',
    '#928374', '#fb4934', '#b8bb26', '#fabd2f', '#83a598', '#d3869b', '#8ec07c', '#ebdbb2',
];

const COLOR_ERROR = '#fb4934';
const COLOR_WARN = '#fabd2f';
const COLOR_DEBUG = '#83a598';
const COLOR_SYSTEM = '#9aa3b2';

function esc(s: string): string {
    return s
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

interface Seg {
    text: string;
    color?: string;
    bold: boolean;
}

/** 解析 ANSI SGR 序列为带色/加粗片段 */
function ansiSegments(text: string): Seg[] {
    const segs: Seg[] = [];
    let fg: string | undefined;
    let bold = false;
    let buf = '';
    const flush = () => {
        if (buf) {
            segs.push({ text: buf, color: fg, bold });
            buf = '';
        }
    };
    let i = 0;
    while (i < text.length) {
        if (text.charCodeAt(i) === 27 && text.charCodeAt(i + 1) === 91) {
            let j = i + 2;
            while (j < text.length && text.charCodeAt(j) < 0x40) j++;
            if (text[j] === 'm') {
                const params = text
                    .slice(i + 2, j)
                    .split(';')
                    .map((x) => parseInt(x, 10));
                for (const p of params) {
                    if (Number.isNaN(p)) continue;
                    if (p === 0) {
                        fg = undefined;
                        bold = false;
                    } else if (p === 1) bold = true;
                    else if (p === 22) bold = false;
                    else if (p >= 30 && p <= 37) fg = PALETTE[p - 30];
                    else if (p >= 90 && p <= 97) fg = PALETTE[p - 90 + 8];
                    else if (p === 39) fg = undefined;
                    // 背景色/256色等暂不处理，直接忽略
                }
            }
            i = j < text.length ? j + 1 : text.length;
            continue;
        }
        if (text[i] === '\x1b') {
            i += 1;
            continue;
        }
        buf += text[i];
        i += 1;
    }
    flush();
    return segs;
}

function segHtml(seg: Seg): string {
    if (!seg.color && !seg.bold) return esc(seg.text);
    const styles: string[] = [];
    if (seg.color) styles.push(`color:${seg.color}`);
    if (seg.bold) styles.push('font-weight:700');
    return `<span style="${styles.join(';')}">${esc(seg.text)}</span>`;
}

/** 从日志行里识别 MC 日志级别（[线程/LEVEL]） */
function detectLevel(text: string): string | null {
    const m = text.match(/\[\s*[^\]\[]*\/(FATAL|ERROR|SEVERE|WARN|DEBUG|TRACE|INFO)\s*\]/i);
    return m ? m[1].toUpperCase() : null;
}

function levelColor(level: string | null): string | undefined {
    if (!level) return undefined;
    if (level === 'ERROR' || level === 'FATAL' || level === 'SEVERE') return COLOR_ERROR;
    if (level === 'WARN') return COLOR_WARN;
    if (level === 'DEBUG' || level === 'TRACE') return COLOR_DEBUG;
    return undefined;
}

/** 渲染单行日志为安全 HTML */
export function renderLogLine(text: string, kind: LogKind): string {
    if (kind === 'system') {
        let color: string | undefined;
        if (text.startsWith('[错误]')) color = COLOR_ERROR;
        else if (text.startsWith('[警告]') || text.startsWith('[提示]')) color = COLOR_WARN;
        else color = COLOR_SYSTEM;
        return `<span style="color:${color}">${esc(text)}</span>`;
    }

    // 游戏输出：优先 ANSI；无颜色码时按日志级别补色
    if (text.includes('\x1b')) {
        return ansiSegments(text).map(segHtml).join('');
    }
    const color = levelColor(detectLevel(text));
    if (color) return `<span style="color:${color}">${esc(text)}</span>`;
    return esc(text);
}
