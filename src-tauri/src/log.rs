use std::fmt;
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
        let now = time::OffsetDateTime::now_utc();
        let time_format = format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
        );
        let time_str = now.format(&time_format).unwrap_or_default();
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
    // 读取编译模式
    let log_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_env_filter(EnvFilter::new(log_level))
        .event_format(PipeFormatter) // 使用自定义格式化器
        .init();
    // 预期输出: tracing::info!("开始下载资源索引..."); -> 2026-08-08 22:31:15.123 | mc_downloader::assets | INFO  | 开始下载资源索引...

    info!(" ______              __  ___                  __  __________       __");
    info!("/_  __/__  ___      /  |/  /__ ____  __ __   /  |/  / ___/ /      / /");
    info!(" / / / _ \\/ _ \\    / /|_/ / _ `/ _ \\/ // /  / /|_/ / /__/ /__    /_/ ");
    info!("/_/  \\___/\\___/   /_/  /_/\\_,_/_//_/\\_, /  /_/  /_/\\___/____/   (_)  ");
    info!("                                   /___/                             ");
}
