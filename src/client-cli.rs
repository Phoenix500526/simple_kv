use futures::StreamExt;
use shellfish::{Command, Shell};
use simple_kv::YamuxCtrl;
use simple_kv::{
    start_client_with_config, ClientConfig, CommandRequest, KvError::InvalidCommand, Kvpair,
};
use std::error::Error;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::info;

#[macro_use]
extern crate shellfish;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    let ctrl = start_client_with_config(&config).await?;
    let mut shell = Shell::new_async(ctrl, "<simple_kv>-$ ");

    shell.commands.insert(
        "HGET",
        Command::new_async(
            "HGET <table> <key>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hget),
        ),
    );
    shell.commands.insert(
        "HMGET",
        Command::new_async(
            "HMGET <table> <key1> <key2> ... <keyN>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hmget),
        ),
    );
    shell.commands.insert(
        "HGETALL",
        Command::new_async(
            "HGETALL <table>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hgetall),
        ),
    );

    shell.commands.insert(
        "HSET",
        Command::new_async(
            "HSET <table> <key> <value>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hset),
        ),
    );

    shell.commands.insert(
        "HMSET",
        Command::new_async(
            "HMSET <table> <key1> <value1> ... <keyN> <valueN>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hmset),
        ),
    );

    shell.commands.insert(
        "HDEL",
        Command::new_async(
            "HDEL <table> <key>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hdel),
        ),
    );

    shell.commands.insert(
        "HMDEL",
        Command::new_async(
            "HMDEL <table> <key1> <key2> ... <keyN>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hmdel),
        ),
    );

    shell.commands.insert(
        "HEXIST",
        Command::new_async(
            "HMHEXISTDEL <table> <key1>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hexist),
        ),
    );

    shell.commands.insert(
        "HMEXIST",
        Command::new_async(
            "HMEXIST <table> <key1> <key2> ... <keyN>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, hmexist),
        ),
    );

    shell.commands.insert(
        "SUBSCRIBE",
        Command::new_async(
            "SUBSCRIBE <channel>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, subscribe),
        ),
    );

    shell.commands.insert(
        "PSUBSCRIBE",
        Command::new_async(
            "PSUBSCRIBE <pattern>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, psubscribe),
        ),
    );

    shell.commands.insert(
        "PUBLISH",
        Command::new_async(
            "PUBLISH <channel> <message>".to_string(),
            async_fn!(YamuxCtrl<TlsStream<TcpStream>>, publish),
        ),
    );

    shell.run_async().await?;

    Ok(())
}

async fn hset(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args.get(1).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HSET <table> <key> <value>".to_string(),
        ))
    })?;
    let key = args.get(2).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HSET <table> <key> <value>".to_string(),
        ))
    })?;
    let value = args.get(3).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HSET <table> <key> <value>".to_string(),
        ))
    })?;
    let cmd = CommandRequest::new_hset(table, key, value.clone().into());
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!("(integer) 1");
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hmset(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args.get(1).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HMSET <table> <key1> <value1> ... <keyN> <valueN>".to_string(),
        ))
    })?;

    let kvpair_iter = args
        .get(2..)
        .ok_or_else(|| {
            Box::new(InvalidCommand(
                "Usage: HMSET <table> <key1> <value1> ... <keyN> <valueN>".to_string(),
            ))
        })?
        .chunks_exact(2);

    if !kvpair_iter.remainder().is_empty() {
        return Err(Box::new(InvalidCommand(
            "The keys' number doesn't match to the values".to_string(),
        )));
    }

    let pairs = kvpair_iter
        .map(|v| Kvpair::new(v[0].clone(), v[1].clone().into()))
        .collect();

    let cmd = CommandRequest::new_hmset(table, pairs);
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!("(integer) {}", data.values.len());
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hget(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HGET <table> <key>".to_string())))?;
    let key = args
        .get(2)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HGET <table> <key>".to_string())))?;
    let cmd = CommandRequest::new_hget(table, key);
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        let value: String = data.values[0].clone().try_into().unwrap();
        info!("GET: {:?}", value);
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hmget(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args.get(1).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HMGET <table> <key1> <key2> ... <keyN>".to_string(),
        ))
    })?;

    let keys = args.get(2..).ok_or_else(|| {
        Box::new(Box::new(InvalidCommand(
            "Usage: HMGET <table> <key1> <key2> ... <keyN>".to_string(),
        )))
    })?;

    println!("{table}");
    let cmd = CommandRequest::new_hmget(table, keys.to_vec());
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!(
            "GET: {:?}",
            data.values
                .into_iter()
                .map(|v| {
                    let value: String = v.try_into().unwrap();
                    value
                })
                .collect::<Vec<_>>()
        );
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hgetall(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HGETALL <table>".to_string())))?;

    let cmd = CommandRequest::new_hgetall(table);
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!(
            "GET: {:?}",
            data.kvpairs
                .into_iter()
                .map(|pair| {
                    let value: String = pair.value.expect("").try_into().unwrap();
                    (pair.key, value)
                })
                .collect::<Vec<_>>()
        );
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hdel(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HDEL <table> <key>".to_string())))?;

    let key = args
        .get(2)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HDEL <table> <key>".to_string())))?;

    let cmd = CommandRequest::new_hdel(table, key);
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!("(integer) 1");
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hmdel(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HDEL <table> <key>".to_string())))?;

    let keys = args.get(2..).ok_or_else(|| {
        Box::new(Box::new(InvalidCommand(
            "Usage: HMDEL <table> <key1> <key2> ... <keyN>".to_string(),
        )))
    })?;

    let cmd = CommandRequest::new_hmdel(table, keys.to_vec());
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!("(integer) {}", data.values.len());
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hexist(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HEXIST <table> <key>".to_string())))?;
    let key = args
        .get(2)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: HEXIST <table> <key>".to_string())))?;
    let cmd = CommandRequest::new_hexist(table, key);
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();

    if data.status == http::StatusCode::OK.as_u16() as u32 {
        let res: bool = data.values[0].clone().try_into().unwrap();
        info!("EXIST: {:?}", res);
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn hmexist(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let table = args.get(1).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: HMEXIST <table> <key1> <key2> ... <keyN>".to_string(),
        ))
    })?;

    let keys = args.get(2..).ok_or_else(|| {
        Box::new(Box::new(InvalidCommand(
            "Usage: HMEXIST <table> <key1> <key2> ... <keyN>".to_string(),
        )))
    })?;

    let cmd = CommandRequest::new_hmexist(table, keys.to_vec());
    let mut stream = ctrl.open_stream().await?;
    let data = stream.execute(&cmd).await.unwrap();
    if data.status == http::StatusCode::OK.as_u16() as u32 {
        info!(
            "EXIST: {:?}",
            data.values
                .into_iter()
                .map(|value| {
                    let value: bool = value.try_into().unwrap();
                    value
                })
                .collect::<Vec<_>>()
        );
    } else {
        info!("{:?}", data.message);
    }
    Ok(())
}

async fn subscribe(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let channel = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: SUBSCRIBE <channel> ".to_string())))?;

    let cmd = CommandRequest::new_subscribe(channel);
    let stream = ctrl.open_stream().await?;
    let mut result = stream.execute_streaming(&cmd).await.unwrap();
    let id = result.id;
    info!("Subscribe topic: {}, id: {}", channel, id);
    while let Some(Ok(data)) = result.next().await {
        info!("Got published data: {:?}", data);
    }

    Ok(())
}

async fn publish(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let channel = args.get(1).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: PUBLISH <channel> <message>".to_string(),
        ))
    })?;

    let message = args.get(2).ok_or_else(|| {
        Box::new(InvalidCommand(
            "Usage: PUBLISH <channel> <message>".to_string(),
        ))
    })?;

    let cmd = CommandRequest::new_publish(channel, vec![message.clone().into()]);
    let mut stream = ctrl.open_stream().await?;
    let result = stream.execute(&cmd).await.unwrap();
    info!("PUBLISH: {:?}", result);
    Ok(())
}

async fn psubscribe(
    ctrl: &mut YamuxCtrl<TlsStream<TcpStream>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let pattern = args
        .get(1)
        .ok_or_else(|| Box::new(InvalidCommand("Usage: PSUBSCRIBE <pattern> ".to_string())))?;

    let cmd = CommandRequest::new_psubscribe(pattern);
    let stream = ctrl.open_stream().await?;
    let mut result = stream.execute_streaming(&cmd).await.unwrap();
    info!("Subscribe pattern: {}, id: {}", pattern, result.id);
    while let Some(Ok(data)) = result.next().await {
        info!("Got published data: {:?}", data);
    }

    Ok(())
}
