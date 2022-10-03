use crate::{CommandResponse, Value};
use dashmap::{DashMap, DashSet};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// topic 里最大存放数据
const BROCASTER_CAPACITY: usize = 128;

/// 下一个 subscription id
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn get_next_subscription_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub trait Topic: Send + Sync + 'static {
    /// 订阅某个主题,这里把 CommandResponse 封装到了 Arc 当中，可以避免在发送的时候进行复制
    fn subscribe(self, name: String) -> mpsc::Receiver<Arc<CommandResponse>>;

    /// 退订某个主题
    fn unsubscribe(self, name: String, id: u32);

    /// 往主题里面发布某内容
    fn publish(self, name: String, value: Arc<CommandResponse>);
}

/// 用于主题发布和订阅的数据结构
#[derive(Default)]
pub struct Broadcaster {
    /// 所有主题列表
    topics: DashMap<String, DashSet<u32>>,
    /// 所有的订阅列表
    subscriptions: DashMap<u32, mpsc::Sender<Arc<CommandResponse>>>,
}

impl Topic for Arc<Broadcaster> {
    fn subscribe(self, name: String) -> mpsc::Receiver<Arc<CommandResponse>> {
        let id = {
            // topics 表中看看有没有 name 对应的 entry，有则获取，没有则创建
            let entry = self.topics.entry(name).or_default();
            let id = get_next_subscription_id();
            entry.value().insert(id);
            id
        };

        // 生成一个 mpsc 的 channel
        let (tx, rx) = mpsc::channel(BROCASTER_CAPACITY);
        let v: Value = (id as i64).into();

        let tx1 = tx.clone();
        // 当你 subscribe 一个 topic 的时候，可以先从其中 receive 相关的 subcribe id
        tokio::spawn(async move {
            if let Err(e) = tx1.send(Arc::new(v.into())).await {
                // TODO: 这个很小概率发生，但目前我们没有善后
                warn!("Failed to send subscription id: {}. Error: {:?}", id, e);
            }
        });

        // 把 tx 存储到 subscription table 中
        self.subscriptions.insert(id, tx);
        debug!("Subscription {} is added", id);

        rx
    }

    fn unsubscribe(self, name: String, id: u32) {
        if let Some(v) = self.topics.get_mut(&name) {
            v.remove(&id);

            if v.is_empty() {
                info!("Topic: {:?} is deleted", &name);
                drop(v);
                self.topics.remove(&name);
            }
        }
        debug!("Subscription {} is removed!", id);
        self.subscriptions.remove(&id);
    }

    fn publish(self, name: String, value: Arc<CommandResponse>) {
        // 使用 tokio 来包装，避免阻塞
        tokio::spawn(async move {
            match self.topics.get(&name) {
                Some(entry) => {
                    let id_table = entry.value().clone();
                    for id in id_table.into_iter() {
                        if let Some(tx) = self.subscriptions.get(&id) {
                            if let Err(e) = tx.send(value.clone()).await {
                                warn!("Publish to {} failed! error: {:?}", id, e);
                            }
                        }
                    }
                }
                None => {}
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_res_ok;
    use std::convert::TryInto;

    #[tokio::test]
    async fn pub_sub_should_work() {
        let b = Arc::new(Broadcaster::default());
        let topic = "lobby".to_string();
        let mut stream1 = b.clone().subscribe(topic.clone());
        let mut stream2 = b.clone().subscribe(topic.clone());
        let v: Value = "hello".into();
        b.clone().publish(topic.clone(), Arc::new(v.clone().into()));

        let id1: i64 = stream1.recv().await.unwrap().as_ref().try_into().unwrap();
        let id2: i64 = stream2.recv().await.unwrap().as_ref().try_into().unwrap();

        assert_ne!(id1, id2);

        let res1 = stream1.recv().await.unwrap();
        let res2 = stream2.recv().await.unwrap();

        assert_eq!(res1, res2);
        assert_res_ok(&res1, &[v.clone()], &[]);

        b.clone().unsubscribe(topic.clone(), id1 as _);

        let v: Value = "world".into();
        b.clone().publish(topic, Arc::new(v.clone().into()));

        let result = stream1.recv().await;
        assert!(result.is_none());
        let res2 = stream2.recv().await.unwrap();
        assert_res_ok(&res2, &[v.clone()], &[]);
    }
}
