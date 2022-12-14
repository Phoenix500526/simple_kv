use anyhow::Result;
use simple_kv::{
    ClientConfig, ClientTlsConfig, GeneralConfig, LevelConfig, LogConfig, RotationConfig,
    ServerConfig, ServerTlsConfig, StorageConfig,
};
use std::fs;

fn main() -> Result<()> {
    const CA_CERT: &str = include_str!("../fixtures/ca.cert");
    const SERVER_CERT: &str = include_str!("../fixtures/server.cert");
    const SERVER_KEY: &str = include_str!("../fixtures/server.key");

    let general_config = GeneralConfig {
        addr: "127.0.0.1:9527".into(),
    };

    let server_config = ServerConfig {
        general: general_config.clone(),
        storage: StorageConfig::SledDb("/tmp/kv_server".into()),
        tls: ServerTlsConfig {
            cert: SERVER_CERT.into(),
            key: SERVER_KEY.into(),
            ca: None,
        },
        log: LogConfig {
            path: "/tmp/kv-log".into(),
            rotation: RotationConfig::Daily,
            level: LevelConfig::Info,
            enable_log_file: true,
            enable_jager: false,
        },
    };

    fs::write(
        "fixtures/server.conf",
        toml::to_string_pretty(&server_config)?,
    )?;

    let client_config = ClientConfig {
        general: general_config,
        tls: ClientTlsConfig {
            domain: "kvserver.acme.inc".into(),
            identity: None,
            ca: Some(CA_CERT.into()),
        },
    };

    fs::write(
        "fixtures/client.conf",
        toml::to_string_pretty(&client_config)?,
    )?;
    Ok(())
}
