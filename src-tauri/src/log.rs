use std::{fmt, panic};
use time::macros::format_description;
use tracing::{Event, Level, Subscriber, info};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

// ANSI 颜色码
const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[34m"; // INFO
const GREEN: &str = "\x1b[32m"; // DEBUG
const YELLOW: &str = "\x1b[33m"; // WARN
const RED: &str = "\x1b[31m"; // ERROR

struct PipeFormatter;

impl<S, N> FormatEvent<S, N> for PipeFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let now = time::OffsetDateTime::now_local();
        let time_format = format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
        );
        let time_str = now.expect("获取系统时间失败").format(&time_format).unwrap_or_default();
        let level = event.metadata().level();
        // 根据级别选择颜色
        let color = match *level {
            Level::INFO => RESET,
            Level::DEBUG => GREEN,
            Level::WARN => YELLOW,
            Level::ERROR => RED,
            Level::TRACE => BLUE,
        };
        let target = event.metadata().target();
        write!(
            writer,
            "{}{} | {} | {:<5} | ",
            color, time_str, target, level
        )?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        write!(writer, "{}", RESET)?;
        writeln!(writer)
    }
}

pub fn init_logger() {
    // 读取编译模式（压制 reqwest 传输栈 h2/hyper/tower 的 DEBUG 噪音，其它模块保持 DEBUG）
    let log_level = if cfg!(debug_assertions) {
        "debug,h2=info,hyper=info,tower=info"
    } else {
        "info,h2=info,hyper=info,tower=info"
    };
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_env_filter(EnvFilter::new(log_level))
        .event_format(PipeFormatter) // 使用自定义格式化器
        .init();
    // 预期输出: tracing::info!("开始下载资源索引..."); -> 2026-08-08 22:31:15.123 | mc_downloader::assets | INFO  | 开始下载资源索引...

    setup_panic_hook();

    info!(" ______              __  ___                  __  __________       __");
    info!("/_  __/__  ___      /  |/  /__ ____  __ __   /  |/  / ___/ /      / /");
    info!(" / / / _ \\/ _ \\    / /|_/ / _ `/ _ \\/ // /  / /|_/ / /__/ /__    /_/ ");
    info!("/_/  \\___/\\___/   /_/  /_/\\_,_/_//_/\\_, /  /_/  /_/\\___/____/   (_)  ");
    info!("                                   /___/                             ");
    info!("Too Many Minecraft Launcher is starting...")
}

/// 将程序 Panic 作为日志输出
fn setup_panic_hook() {
    panic::set_hook(Box::new(move |info| {
        // 提取 panic 发生的位置（文件名:行号）
        let location = info.location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());

        // 提取 panic 的消息内容
        let payload = info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&'static str>() {
            // 匹配 panic!("literal string") 或 expect("literal string")
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            // 匹配 panic!("formatted {}", value)
            s.clone()
        } else {
            "未知 Panic".to_string()
        };

        // 通过 tracing 输出 error 级别日志
        tracing::error!(
            "{} | 程序抛出 Panic 导致崩溃！{}", location, message
        );
    }));
}
