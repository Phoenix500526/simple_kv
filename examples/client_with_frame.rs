use anyhow::Result;
use bytes::Bytes;
use futures::prelude::*;
use prost::Message;
use simple_kv::CommandRequest;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).await?;

    let mut stream = Framed::new(stream, LengthDelimitedCodec::new());

    // 生成相应的命令并发送
    let cmd = CommandRequest::new_hset("table1", "k1", "v1".into());
    stream.send(Bytes::from(cmd.encode_to_vec())).await?;

    if let Some(Ok(data)) = stream.next().await {
        info!("Got response {:?}", data);
    }
    Ok(())
}
