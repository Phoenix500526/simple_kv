use anyhow::Result;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use prost::Message;
use simple_kv::{CommandRequest, Service, ServiceInner, SledDb};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let service: Service<SledDb> = ServiceInner::new(SledDb::new("/tmp/kvserver"))
        .fn_before_send(|res| match res.message.as_ref() {
            "" => {
                res.message = "altered. Original message is empty.".into();
                Ok(())
            }
            s => {
                res.message = format!("altered: {}", s);
                Ok(())
            }
        })
        .into();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
        info!("Client {:?} connected", addr);
        let svc = service.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let cmd = CommandRequest::decode(msg).unwrap();
                info!("Got a new command: {:?}", cmd);
                let mut res = svc.execute(cmd);
                while let Some(data) = res.next().await {
                    stream
                        .send(Bytes::from((*data).clone().encode_to_vec()))
                        .await
                        .unwrap();
                }
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
