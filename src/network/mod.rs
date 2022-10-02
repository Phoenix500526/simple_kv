mod frame;
use bytes::BytesMut;
pub use frame::{read_frame, FrameCoder};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::info;

use crate::{CommandRequest, CommandResponse, KvError, Service};

/// 处理服务端某个 accept 下的 socket 读写
pub struct ProstServerStream<S> {
    inner: S,
    service: Service,
}

/// 处理 Client socket 的读写
pub struct ProstClientStream<S> {
    inner: S,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: stream,
            service,
        }
    }

    // process 是对外的方法
    pub async fn process(mut self) -> Result<(), KvError> {
        while let Ok(cmd) = self.recv().await {
            info!("Got a new command: {:?}", cmd);
            let res = self.service.execute(cmd);
            self.send(res).await?;
        }
        Ok(())
    }

    async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf).unwrap();
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..]).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandRequest, KvError> {
        let mut buf = BytesMut::new();
        read_frame(&mut self.inner, &mut buf).await?;
        CommandRequest::decode_frame(&mut buf)
    }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> ProstClientStream<S> {
        Self { inner: stream }
    }

    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        self.send(cmd).await?;
        self.recv().await
    }

    async fn send(&mut self, cmd: CommandRequest) -> Result<(), KvError> {
        let mut buf = BytesMut::new();
        cmd.encode_frame(&mut buf).unwrap();
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..]).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandResponse, KvError> {
        let mut buf = BytesMut::new();
        read_frame(&mut self.inner, &mut buf).await?;
        CommandResponse::decode_frame(&mut buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::{assert_res_ok, MemTable, ServiceInner, Value};

    use super::*;
    use anyhow::Result;
    use bytes::Bytes;
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });
        Ok(addr)
    }

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute(cmd).await.unwrap();

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await.unwrap();

        assert_res_ok(res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t1", "k1", v.clone());
        let res = client.execute(cmd).await.unwrap();

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await.unwrap();
        assert_res_ok(res, &[v], &[]);
        Ok(())
    }
}
