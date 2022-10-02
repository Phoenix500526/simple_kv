use anyhow::Result;
use simple_kv::{CommandRequest, ProstClientStream, TlsClientConnector};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let ca_cert = include_str!("../fixtures/ca.cert");
    let client_cert = include_str!("../fixtures/client.cert");
    let client_key = include_str!("../fixtures/client.key");
    let client_identity = Some((client_cert, client_key));
    let addr = "127.0.0.1:9527";
    let connector = TlsClientConnector::new("kvserver.acme.inc", client_identity, Some(ca_cert))?;
    let stream = TcpStream::connect(addr).await?;
    let stream = connector.connect(stream).await?;

    let mut client = ProstClientStream::new(stream);
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());
    let res = client.execute(cmd).await?;
    info!("Got response {:?}", res);
    Ok(())
}
