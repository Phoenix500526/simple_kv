use anyhow::Result;
use simple_kv::{
    start_client_with_config, start_server_with_config, ClientConfig, CommandRequest, ServerConfig,
    StorageConfig,
};
use std::time::Duration;
use tokio::time;

#[tokio::test]
async fn yamux_server_client_full_tests() -> Result<()> {
    let addr = "127.0.0.1:10086";
    let mut config: ServerConfig = toml::from_str(include_str!("../fixtures/server.conf"))?;
    config.general.addr = addr.into();
    config.storage = StorageConfig::MemTable;

    // 启动 server
    tokio::spawn(async move {
        start_server_with_config(&config).await.unwrap();
    });

    time::sleep(Duration::from_millis(10)).await;
    let mut config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    config.general.addr = addr.into();

    let mut ctrl = start_client_with_config(&config).await.unwrap();
    let mut stream = ctrl.open_stream().await?;

    // 生成一个 HSET 命令
    let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
    stream.execute(&cmd).await?;

    // 生成一个 HGET 命令
    let cmd = CommandRequest::new_hget("t1", "k1");
    let res = stream.execute(&cmd).await?;

    // 集成测试和 crate 使用同样的条件编译，因此无法使用单元测试中构建的辅助代码
    assert_eq!(res.status, 200);
    assert_eq!(res.values, &["v1".into()]);
    Ok(())
}
