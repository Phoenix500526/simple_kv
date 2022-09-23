use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use simple_kv::{CommandResponse, CommandRequest};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).await?;

    // 使用 AsyncProstStream 处理 TCP Frame
    let mut client = AsyncProstStream::<_, CommandResponse, CommandRequest, _>::from(stream).for_async();

    // 生成相应的命令并发送
    let cmd = CommandRequest::new_hset("table1", "k1", "v1".into());
    client.send(cmd).await?;

    if let Some(Ok(data)) = client.next().await {
        info!("Got response {:?}", data);
    }
    Ok(())
}
