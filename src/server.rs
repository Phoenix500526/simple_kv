use anyhow::Result;
use simple_kv::{start_server_with_config, LevelConfig, RotationConfig, ServerConfig};
use std::env;
use tokio::fs;
use tracing::span;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format},
    layer::SubscriberExt,
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config_file = match env::var("KV_SERVER_CONFIG") {
        Ok(path) => fs::read_to_string(&path).await?,
        Err(_) => include_str!("../fixtures/server.conf").to_string(),
    };
    let config: ServerConfig = toml::from_str(&config_file)?;
    let log = &config.log;

    let level = match log.level {
        LevelConfig::Trace => LevelFilter::TRACE,
        LevelConfig::Debug => LevelFilter::DEBUG,
        LevelConfig::Info => LevelFilter::INFO,
        LevelConfig::Warn => LevelFilter::WARN,
        LevelConfig::Error => LevelFilter::ERROR,
    };
    let level_layer = fmt::layer().with_filter(level);

    let subscribe = tracing_subscriber::registry().with(level_layer);

    let fmt_layer = if log.enable_log_file {
        let file_appender = match log.rotation {
            RotationConfig::Hourly => tracing_appender::rolling::hourly(&log.path, "server.log"),
            RotationConfig::Daily => tracing_appender::rolling::daily(&log.path, "server.log"),
            RotationConfig::Never => tracing_appender::rolling::never(&log.path, "server.log"),
        };

        let (non_blocking, _guard1) = tracing_appender::non_blocking(file_appender);
        let fmt_layer = fmt::layer()
            .event_format(format().compact())
            .with_writer(non_blocking)
            .with_filter(level);

        Some(fmt_layer)
    } else {
        None
    };

    let jager_layer = if log.enable_jager {
        let tracer = opentelemetry_jaeger::new_pipeline()
            .with_service_name("kv-server")
            .install_simple()?;

        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        Some(opentelemetry)
    } else {
        None
    };

    subscribe.with(fmt_layer).with(jager_layer).init();

    let root = span!(tracing::Level::INFO, "app_strat", work_units = 2);
    let _enter = root.enter();

    start_server_with_config(&config).await?;
    Ok(())
}
