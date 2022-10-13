use anyhow::Result;
use criterion::{criterion_group, criterion_main, Criterion};
use futures::StreamExt;
use rand::seq::SliceRandom;
use simple_kv::{
    start_client_with_config, start_server_with_config, ClientConfig, CommandRequest, ServerConfig,
    StorageConfig, YamuxCtrl,
};
use tokio::{net::TcpStream, runtime::Builder, time, time::Duration};
use tokio_rustls::client::TlsStream;
use tracing::{info, span};
use tracing_subscriber::{layer::SubscriberExt, prelude::*, EnvFilter};

async fn start_server() -> Result<()> {
    let addr = "127.0.0.1:8820";
    let mut config: ServerConfig = toml::from_str(include_str!("../fixtures/server.conf"))?;
    config.general.addr = addr.into();
    config.storage = StorageConfig::MemTable;
    tokio::spawn(async move {
        start_server_with_config(&config).await.unwrap();
    });
    Ok(())
}

async fn connect() -> Result<YamuxCtrl<TlsStream<TcpStream>>> {
    let addr = "127.0.0.1:8820";
    let mut config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    config.general.addr = addr.into();
    start_client_with_config(&config).await
}

async fn start_subscriber(topic: &'static str) -> Result<()> {
    let mut ctrl = connect().await?;
    let stream = ctrl.open_stream().await?;
    info!("C(subscriber): stream opened");
    let cmd = CommandRequest::new_subscribe(topic.to_string());
    tokio::spawn(async move {
        let mut stream = stream.execute_streaming(&cmd).await.unwrap();
        while let Some(Ok(data)) = stream.next().await {
            drop(data);
        }
    });
    Ok(())
}

async fn start_publisher(topic: &'static str, values: &'static [&'static str]) -> Result<()> {
    let mut rng = rand::thread_rng();
    let v = values.choose(&mut rng).unwrap();
    let mut ctrl = connect().await.unwrap();
    let mut stream = ctrl.open_stream().await.unwrap();
    info!("C(publisher): stream opened");
    let cmd = CommandRequest::new_publish(topic.to_string(), vec![(*v).into()]);
    stream.execute(&cmd).await.unwrap();
    Ok(())
}

fn pubsub(c: &mut Criterion) {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("kv-bench")
        .install_simple()
        .unwrap();

    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(opentelemetry)
        .init();

    let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    let _enter = root.enter();
    // 创建 tokio runtime。由于我们所需要测试的函数(start_publisher, start_subscriber) 都是 async 函数
    // 因此我们需要先使用 runtime，普通函数则不需要
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("pubsub")
        .enable_all()
        .build()
        .unwrap();

    // 准备测试数据
    let values = &["Hello", "Tyr", "Goodbye", "World"];
    let topic = "lobby";

    // 做测试准备：运行服务器和 100 个 subscriber
    runtime.block_on(async {
        eprint!("preparing server and subscribers");
        start_server().await.unwrap();
        time::sleep(Duration::from_millis(50)).await;
        for _ in 0..100 {
            start_subscriber(topic).await.unwrap();
            eprint!(".");
        }
        eprintln!("Done!");
    });

    // 进行 benchmark
    c.bench_function("publishing", move |b| {
        b.to_async(&runtime)
            .iter(|| async { start_publisher(topic, values).await })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = pubsub
}
criterion_main!(benches);
