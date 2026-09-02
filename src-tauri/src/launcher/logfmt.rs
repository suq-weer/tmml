//! Minecraft 控制台输出的解析与格式化。
//!
//! 现代原版通过 log4j2 的 `LegacyXMLLayout`（下载的 assets/log_configs/*.xml 即此布局）
//! 向控制台输出 XML 事件流，形如：
//!
//! ```xml
//! <log4j:Event logger="net.minecraft.client.Minecraft" timestamp="..." level="INFO" thread="Render thread">
//! <log4j:Message><![CDATA[Stopping!]]></log4j:Message>
//! </log4j:Event>
//! ```
//!
//! 一个事件可能跨多行（含 CDATA 里的换行），因此先把“<log4j:Event>…</log4j:Event>”
//! 整段组装出来，再解析为可读的 `[HH:mm:ss] [thread/LEVEL]: message` 终端行，
//! 让 Island 日志视图既能正确断行，也能按级别上色。

/// 事件开标签（大小写不敏感查找用）
pub const EVENT_START: &[u8] = b"<log4j:event";
/// 事件结束标签
pub const EVENT_END: &[u8] = b"</log4j:event>";

/// 大小写不敏感的字节子串查找
pub fn find_ci(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

/// 第一个事件开标签 `<log4j:Event` 的位置（无则 None）
pub fn find_event_start(bytes: &[u8]) -> Option<usize> {
    find_ci(bytes, EVENT_START)
}

/// 从某位置开始查找完整事件结束标签，返回“结束标签之后”的字节偏移
pub fn find_event_end_after(bytes: &[u8]) -> Option<usize> {
    find_ci(bytes, EVENT_END).map(|i| i + EVENT_END.len())
}

/// 定位完整事件块 [start, end)：以 `<log4j:Event` 开头、`</log4j:Event>` 结尾
pub fn locate_event(bytes: &[u8]) -> Option<(usize, usize)> {
    let start = find_event_start(bytes)?;
    let after = find_event_end_after(&bytes[start..])?;
    Some((start, start + after))
}

/// 解析出的单个日志事件
#[derive(Debug, Clone)]
pub struct GameLogEvent {
    pub timestamp_ms: i64,
    pub level: String,
    pub thread: String,
    pub logger: String,
    pub message: String,
    /// 消息之外的内容（如 <log4j:Throwable> 里的堆栈），按行附加
    pub extra_lines: Vec<String>,
}

/// 将一段 XML 事件解析为可读事件
pub fn parse_event_block(xml: &str) -> Option<GameLogEvent> {
    let bytes = xml.as_bytes();
    let start = find_ci(bytes, EVENT_START)?;
    let open_end_rel = xml[start..].find('>')?;
    let open_end = start + open_end_rel;
    let attr_str = &xml[start + EVENT_START.len()..open_end];
    let attrs = parse_attrs(attr_str);

    let end = find_ci(bytes, EVENT_END)?;
    let content = &xml[open_end + 1..end];

    // Message 元素内容
    if let Some(msg_start_rel) = find_ci(content.as_bytes(), b"<log4j:message") {
        let tag_end_rel = content[msg_start_rel..].find('>')? + msg_start_rel;
        let tag_end = tag_end_rel + 1;
        if let Some(msg_close_rel) = find_ci(content.as_bytes(), b"</log4j:message") {
            let raw = &content[tag_end..msg_close_rel];
            let message = decode_xml_text(raw.trim_start());
            // 事件里 Message 之后的其余内容（Throwable/NDC 等）作为附加文本
            let close_len = b"</log4j:message>".len();
            let rest = &content[msg_close_rel + close_len..];
            let extra_text = extract_text_ignoring_tags(rest);
            let extra_lines: Vec<String> = extra_text
                .lines()
                .map(str::trim_end)
                .filter(|l| !l.trim().is_empty())
                .map(str::to_string)
                .collect();
            return Some(GameLogEvent {
                timestamp_ms: attrs.get("timestamp").and_then(|s| s.parse().ok()).unwrap_or(0),
                level: attrs.get("level").cloned().unwrap_or_default(),
                thread: attrs.get("thread").cloned().unwrap_or_default(),
                logger: attrs.get("logger").cloned().unwrap_or_default(),
                message,
                extra_lines,
            });
        }
    }

    // 没有 Message 元素：退化为整体文本（异常情况尽量不丢信息）
    Some(GameLogEvent {
        timestamp_ms: attrs.get("timestamp").and_then(|s| s.parse().ok()).unwrap_or(0),
        level: attrs.get("level").cloned().unwrap_or_default(),
        thread: attrs.get("thread").cloned().unwrap_or_default(),
        logger: attrs.get("logger").cloned().unwrap_or_default(),
        message: decode_xml_text(content.trim_start()),
        extra_lines: Vec::new(),
    })
}

/// 把事件转成若干行可读文本：首行带时间/线程/级别头，后续为消息的其余行与异常行
pub fn event_to_lines(ev: &GameLogEvent) -> Vec<String> {
    let time = format_timestamp_ms(ev.timestamp_ms);
    let level = ev.level.to_uppercase();
    let thread = if ev.thread.is_empty() { "?" } else { ev.thread.trim() };

    let mut lines = Vec::new();
    let header = format!("[{}] [{}/{}]", time, thread, level);
    let msg_lines: Vec<&str> = ev.message.lines().map(str::trim_end).collect();
    if msg_lines.is_empty() {
        lines.push(format!("{}: ", header));
    } else {
        lines.push(format!("{}: {}", header, msg_lines[0]));
        lines.extend(msg_lines[1..].iter().map(|l| (*l).to_string()));
    }
    lines.extend(ev.extra_lines.iter().cloned());
    lines
}

fn parse_attrs(s: &str) -> std::collections::HashMap<String, String> {
    let bytes = s.as_bytes();
    let mut attrs = std::collections::HashMap::new();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        while i < n && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        // 属性名：一直读到 '='
        let name_start = i;
        while i < n && bytes[i] != b'=' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let name = s[name_start..i].trim().to_string();
        i += 1; // 跳过 '='
        while i < n && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = if i < n && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < n && bytes[i] != quote {
                i += 1;
            }
            let value = decode_entities(&s[value_start..i]);
            if i < n {
                i += 1;
            }
            value
        } else {
            let value_start = i;
            while i < n && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            decode_entities(&s[value_start..i])
        };
        attrs.insert(name, value);
    }
    attrs
}

/// 解包 CDATA 或做基本实体反转义
fn decode_xml_text(raw: &str) -> String {
    let raw = raw.trim_start();
    if let Some(rest) = raw.strip_prefix("<![CDATA[") {
        return match rest.find("]]>") {
            Some(idx) => rest[..idx].to_string(),
            None => rest.to_string(),
        };
    }
    decode_entities(raw)
}

/// 忽略标签，仅保留其中所有文本（含 CDATA），用于异常堆栈等附加内容
fn extract_text_ignoring_tags(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while !rest.is_empty() {
        match rest.find('<') {
            None => {
                out.push_str(rest);
                break;
            }
            Some(idx) => {
                out.push_str(&rest[..idx]);
                let after = &rest[idx..];
                if let Some(cdata) = after.strip_prefix("<![CDATA[") {
                    if let Some(end) = cdata.find("]]>") {
                        out.push_str(&cdata[..end]);
                        rest = &cdata[end + 3..];
                        continue;
                    }
                    out.push_str(cdata);
                    rest = "";
                    continue;
                }
                // 普通标签：跳过到 '>'
                match after.find('>') {
                    Some(tag_end) => rest = &after[tag_end + 1..],
                    None => rest = "",
                }
            }
        }
    }
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if s[i..].starts_with("&amp;") {
                out.push('&');
                i += 5;
                continue;
            }
            if s[i..].starts_with("&lt;") {
                out.push('<');
                i += 4;
                continue;
            }
            if s[i..].starts_with("&gt;") {
                out.push('>');
                i += 4;
                continue;
            }
            if s[i..].starts_with("&quot;") {
                out.push('"');
                i += 6;
                continue;
            }
            if s[i..].starts_with("&apos;") {
                out.push('\'');
                i += 6;
                continue;
            }
            if s[i..].starts_with("&#") {
                if let Some(semi) = s[i + 2..].find(';') {
                    if let Ok(code) = s[i + 2..i + 2 + semi].parse::<u32>() {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                            i += 2 + semi + 1;
                            continue;
                        }
                    }
                }
            }
            out.push('&');
            i += 1;
            continue;
        }
        // UTF-8：按字符前进
        let ch_len = utf8_len(bytes[i]);
        let end = (i + ch_len).min(bytes.len());
        out.push_str(&s[i..end]);
        i = end;
    }
    out
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// 将 epoch 毫秒格式化为本地时间 HH:mm:ss；失败回退空串
pub fn format_timestamp_ms(ms: i64) -> String {
    let nanos = (ms as i128) * 1_000_000;
    let Ok(odt) = time::OffsetDateTime::from_unix_timestamp_nanos(nanos) else {
        return String::new();
    };
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let local = odt.to_offset(offset);
    local
        .format(&time::macros::format_description!("[hour]:[minute]:[second]"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "<log4j:Event logger=\"net.minecraft.client.Minecraft\" ",
        "timestamp=\"1788355881585\" level=\"INFO\" thread=\"Render thread\">\n",
        "<log4j:Message><![CDATA[Stopping!]]></log4j:Message>\n",
        "</log4j:Event>",
    );

    #[test]
    fn locate_single_event() {
        let bytes = format!("{}\n{}", "junk\n", SAMPLE);
        let (s, e) = locate_event(bytes.as_bytes()).expect("应定位到事件");
        assert_eq!(&bytes[s..e], SAMPLE);
    }

    #[test]
    fn parse_sample_event() {
        let ev = parse_event_block(SAMPLE).expect("解析失败");
        assert_eq!(ev.logger, "net.minecraft.client.Minecraft");
        assert_eq!(ev.level, "INFO");
        assert_eq!(ev.thread, "Render thread");
        assert_eq!(ev.message, "Stopping!");
        assert!(ev.extra_lines.is_empty());
    }

    #[test]
    fn render_readable_line() {
        let ev = parse_event_block(SAMPLE).unwrap();
        let lines = event_to_lines(&ev);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("[Render thread/INFO]: Stopping!"));
        assert!(lines[0].starts_with("["));
    }

    #[test]
    fn multiline_message_preserved() {
        let xml = "<log4j:Event logger=\"a\" timestamp=\"0\" level=\"ERROR\" thread=\"t\">\n\
                   <log4j:Message><![CDATA[first\nsecond]]></log4j:Message>\n\
                   </log4j:Event>";
        let ev = parse_event_block(xml).unwrap();
        assert_eq!(ev.message, "first\nsecond");
        let lines = event_to_lines(&ev);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with("second"));
    }

    #[test]
    fn throwable_collected_as_extra() {
        let xml = "<log4j:Event logger=\"a\" timestamp=\"0\" level=\"ERROR\" thread=\"main\">\n\
                   <log4j:Message><![CDATA[boom]]></log4j:Message>\n\
                   <log4j:Throwable><![CDATA[java.lang.Exception: x\n\tat foo.Bar.main(Bar.java:1)]]></log4j:Throwable>\n\
                   </log4j:Event>";
        let ev = parse_event_block(xml).unwrap();
        assert_eq!(ev.message, "boom");
        assert!(ev.extra_lines.join("\n").contains("at foo.Bar.main"));
    }

    #[test]
    fn escaped_message_decoded() {
        let xml = "<log4j:Event logger=\"a\" timestamp=\"0\" level=\"INFO\" thread=\"t\">\
                   <log4j:Message>a &amp;&lt;b&gt;</log4j:Message>\
                   </log4j:Event>";
        let ev = parse_event_block(xml).unwrap();
        assert_eq!(ev.message, "a &<b>");
    }
}
