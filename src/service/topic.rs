use crate::{CommandResponse, KvError, Value};
use dashmap::{DashMap, DashSet};
use glob::Pattern;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};

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
    fn unsubscribe(self, name: String, id: u32) -> Result<u32, KvError>;

    /// 往主题里面发布某内容
    fn publish(self, name: String, value: Arc<CommandResponse>);

    /// 订阅某个模式
    fn psubscribe(self, pattern: &'static str) -> mpsc::Receiver<Arc<CommandResponse>>;

    /// 退订某个模式
    fn punsubscribe(self, pattern: &'static str, id: u32) -> Result<u32, KvError>;
}

/// 用于主题发布和订阅的数据结构
#[derive(Default)]
pub struct Broadcaster {
    /// 所有主题列表
    topics: DashMap<String, DashSet<u32>>,
    /// 所有的订阅列表
    subscriptions: DashMap<u32, mpsc::Sender<Arc<CommandResponse>>>,
    /// 所有的模式订阅列表
    patterns: DashMap<Pattern, DashSet<u32>>,
}

impl Broadcaster {
    fn remove_subscription(&self, name: String, id: u32) -> Option<u32> {
        if let Some(v) = self.topics.get_mut(&name) {
            v.remove(&id);

            if v.is_empty() {
                info!("Topic: {:?} is deleted", &name);
                drop(v);
                self.topics.remove(&name);
            }
        }
        debug!("Subscription {} is removed!", id);
        self.subscriptions.remove(&id).map(|(id, _)| id)
    }

    fn remove_pattern(&self, pattern: Pattern, id: u32) -> Option<u32> {
        if let Some(v) = self.patterns.get_mut(&pattern) {
            v.remove(&id);

            if v.is_empty() {
                info!("Topic: {:?} is deleted", &pattern);
                drop(v);
                self.patterns.remove(&pattern);
            }
        }
        debug!("Subscription {} is removed!", id);
        self.subscriptions.remove(&id).map(|(id, _)| id)
    }
}

impl Topic for Arc<Broadcaster> {
    #[instrument(name = "topic_subscribe", skip_all)]
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

    #[instrument(name = "pattern_subscribe", skip_all)]
    fn psubscribe(self, pattern: &'static str) -> mpsc::Receiver<Arc<CommandResponse>> {
        let pattern = Pattern::new(pattern).unwrap();
        let id = {
            // topics 表中看看有没有 name 对应的 entry，有则获取，没有则创建
            let entry = self.patterns.entry(pattern).or_default();
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

    #[instrument(name = "topic_unsubscribe", skip_all)]
    fn unsubscribe(self, name: String, id: u32) -> Result<u32, KvError> {
        match self.remove_subscription(name, id) {
            Some(v) => Ok(v),
            None => Err(KvError::NotFound(format!("subscription {}", id))),
        }
    }

    #[instrument(name = "pattern_unsubscribe", skip_all)]
    fn punsubscribe(self, pattern: &'static str, id: u32) -> Result<u32, KvError> {
        let pattern = Pattern::new(pattern).unwrap();
        match self.remove_pattern(pattern, id) {
            Some(v) => Ok(v),
            None => Err(KvError::NotFound(format!("pattern {}", id))),
        }
    }

    #[instrument(name = "topic_publish", skip_all)]
    fn publish(self, name: String, value: Arc<CommandResponse>) {
        // 使用 tokio 来包装，避免阻塞
        tokio::spawn(async move {
            let mut ids = vec![];
            if let Some(topic) = self.topics.get(&name) {
                // 复制整个 topic 下所有的 subscription id
                // 这里我们每个 id 是 u32，如果一个 topic 下有 10k 订阅，复制的成本
                // 也就是 40k 堆内存（外加一些控制结构），所以效率不算差
                // 这也是为什么我们用 NEXT_ID 来控制 subscription id 的生成
                let subscriptions = topic.value().clone();

                // 尽快释放锁
                drop(topic);
                for id in subscriptions.into_iter() {
                    if let Some(tx) = self.subscriptions.get(&id) {
                        if let Err(e) = tx.send(value.clone()).await {
                            warn!("Publish to {} failed! error: {:?}", id, e);
                            // client 中断连接
                            ids.push(id);
                        }
                    }
                }

                for id in ids {
                    self.remove_subscription(name.clone(), id);
                }
            }

            let mut pattern_ids = vec![];

            let patter_matched = self
                .patterns
                .iter()
                .filter_map(|pair| match pair.key().matches(&name[..]) {
                    true => Some(pair),
                    false => None,
                })
                .collect::<Vec<_>>();

            for element in patter_matched {
                for id in element.value().clone().into_iter() {
                    if let Some(tx) = self.subscriptions.get(&id) {
                        if let Err(e) = tx.send(value.clone()).await {
                            warn!("Publish to {} failed! error: {:?}", id, e);
                            // client 中断连接
                            pattern_ids.push((element.key().clone(), id));
                        }
                    }
                }
            }

            for (pattern, id) in pattern_ids {
                self.remove_pattern(pattern, id);
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

        let result = b.clone().unsubscribe(topic.clone(), id1 as _).unwrap();
        assert_eq!(result, id1 as u32);

        let v: Value = "world".into();
        b.clone().publish(topic, Arc::new(v.clone().into()));

        let result = stream1.recv().await;
        assert!(result.is_none());
        let res2 = stream2.recv().await.unwrap();
        assert_res_ok(&res2, &[v.clone()], &[]);
    }

    #[tokio::test]
    async fn pattern_pub_sub_should_work() {
        let b = Arc::new(Broadcaster::default());
        let topic_1 = "chat.rust".to_string();
        let topic_2 = "chat.exilir".to_string();
        let topic_3 = "chat".to_string();

        let mut stream1 = b.clone().subscribe(topic_1);
        let mut stream2 = b.clone().subscribe(topic_2);
        let mut stream3 = b.clone().subscribe(topic_3);
        let mut stream4 = b.clone().psubscribe("chat.*");
        let id1: i64 = stream1.recv().await.unwrap().as_ref().try_into().unwrap();
        let id2: i64 = stream2.recv().await.unwrap().as_ref().try_into().unwrap();
        let id3: i64 = stream3.recv().await.unwrap().as_ref().try_into().unwrap();
        let id4: i64 = stream4.recv().await.unwrap().as_ref().try_into().unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id3, id4);
        let v1: Value = "chat.rust".into();
        let v2: Value = "chat.exilir".into();
        let v3: Value = "chat".into();

        b.clone()
            .publish("chat.rust".to_string(), Arc::new(v1.clone().into()));
        b.clone()
            .publish("chat.exilir".to_string(), Arc::new(v2.clone().into()));
        b.clone()
            .publish("chat".to_string(), Arc::new(v3.clone().into()));

        let result1 = stream1.recv().await.unwrap();
        let result2 = stream2.recv().await.unwrap();
        let result3 = stream3.recv().await.unwrap();
        let result4 = stream4.recv().await.unwrap();
        assert_res_ok(&result1, &[v1.clone()], &[]);
        assert_res_ok(&result2, &[v2.clone()], &[]);
        assert_res_ok(&result3, &[v3.clone()], &[]);
        assert_res_ok(&result4, &[v1.clone()], &[]);
        let result4 = stream4.recv().await.unwrap();
        assert_res_ok(&result4, &[v2.clone()], &[]);

        b.clone().punsubscribe("chat.*", id4 as _).unwrap();
        b.clone()
            .publish("chat.exilir".to_string(), Arc::new(v2.clone().into()));

        let result = stream4.recv().await;
        assert!(result.is_none());
        let res2 = stream2.recv().await.unwrap();
        assert_res_ok(&res2, &[v2.clone()], &[]);
    }
}
