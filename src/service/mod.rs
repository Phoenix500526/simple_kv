use crate::command_request::*;
use crate::*;
use futures::stream;
use std::sync::Arc;
use tracing::debug;
mod command_service;
mod topic;
mod topic_service;
pub use self::{
    topic::{Broadcaster, Topic},
    topic_service::{StreamingResponse, TopicService},
};

/// 对 Command 的处理进行抽象
pub trait CommandService {
    /// 处理 Command 返回 Response
    fn execute(self, store: &impl Storage) -> CommandResponse;
}

/// Service 数据结构，其作用是将 CommandService 和 Storage 这两个 Trait 联合起来
/// 避免向用户暴露底层细节
pub struct Service<Store = MemTable> {
    inner: Arc<ServiceInner<Store>>,
    brocaster: Arc<Broadcaster>,
}

impl<Store> Clone for Service<Store> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            brocaster: Arc::clone(&self.brocaster),
        }
    }
}

pub type ReceivedFunc = fn(&CommandRequest) -> Result<(), KvError>;
pub type ResponseFunc = fn(&CommandResponse) -> Result<(), KvError>;
pub type BeforeSendFunc = fn(&mut CommandResponse) -> Result<(), KvError>;
pub type AfterSendFunc = fn() -> Result<(), KvError>;

/// ServiceInner：Service 内部数据结构，这也是 Rust 的一个惯例，把需要在多线程下 clone 的主体和其内部结构分开，这样代码逻辑更加清晰
pub struct ServiceInner<Store> {
    store: Store,
    on_received: Vec<ReceivedFunc>,
    on_executed: Vec<ResponseFunc>,
    on_before_send: Vec<BeforeSendFunc>,
    on_after_send: Vec<AfterSendFunc>,
}

impl<Store: Storage> Service<Store> {
    pub fn execute(&self, cmd: CommandRequest) -> StreamingResponse {
        debug!("Got request: {:?}", cmd);
        if let Err(e) = self.inner.on_received.notify(&cmd) {
            return Box::pin(stream::once(async { Arc::new(e) }));
        }
        let mut res = dispatch(cmd.clone(), &self.inner.store);

        if res == CommandResponse::default() {
            dispatch_stream(cmd, Arc::clone(&self.brocaster))
        } else {
            debug!("Executed response: {:?}", res);
            if let Err(e) = self.inner.on_executed.notify(&res) {
                return Box::pin(stream::once(async { Arc::new(e) }));
            }
            if let Err(e) = self.inner.on_before_send.notify(&mut res) {
                return Box::pin(stream::once(async { Arc::new(e) }));
            }
            if !self.inner.on_after_send.is_empty() {
                debug!("Modified response: {:?}", res);
            }
            Box::pin(stream::once(async { Arc::new(res) }))
        }
    }
}

impl<Store: Storage> ServiceInner<Store> {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            on_received: Vec::new(),
            on_executed: Vec::new(),
            on_before_send: Vec::new(),
            on_after_send: Vec::new(),
        }
    }

    pub fn fn_received(mut self, f: fn(&CommandRequest) -> Result<(), KvError>) -> Self {
        self.on_received.push(f);
        self
    }

    pub fn fn_executed(mut self, f: fn(&CommandResponse) -> Result<(), KvError>) -> Self {
        self.on_executed.push(f);
        self
    }

    pub fn fn_before_send(mut self, f: fn(&mut CommandResponse) -> Result<(), KvError>) -> Self {
        self.on_before_send.push(f);
        self
    }

    pub fn fn_after_send(mut self, f: fn() -> Result<(), KvError>) -> Self {
        self.on_after_send.push(f);
        self
    }
}

impl<Store: Storage> From<ServiceInner<Store>> for Service<Store> {
    fn from(inner: ServiceInner<Store>) -> Self {
        Self {
            inner: Arc::new(inner),
            brocaster: Default::default(),
        }
    }
}

/// 不可变事件通知
pub trait Notify<Arg> {
    fn notify(&self, arg: &Arg) -> Result<(), CommandResponse>;
}

/// 可变事件通知
pub trait NotifyMut<Arg> {
    fn notify(&self, arg: &mut Arg) -> Result<(), CommandResponse>;
}

impl<Arg> Notify<Arg> for Vec<fn(&Arg) -> Result<(), KvError>> {
    #[inline]
    fn notify(&self, arg: &Arg) -> Result<(), CommandResponse> {
        for f in self {
            if let Err(e) = f(arg) {
                return Err(e.into());
            }
        }
        Ok(())
    }
}

impl<Arg> NotifyMut<Arg> for Vec<fn(&mut Arg) -> Result<(), KvError>> {
    #[inline]
    fn notify(&self, arg: &mut Arg) -> Result<(), CommandResponse> {
        for f in self {
            if let Err(e) = f(arg) {
                return Err(e.into());
            }
        }
        Ok(())
    }
}

pub fn dispatch(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
    match cmd.request_data {
        Some(RequestData::Hget(v)) => v.execute(store),
        Some(RequestData::Hmget(v)) => v.execute(store),
        Some(RequestData::Hgetall(v)) => v.execute(store),
        Some(RequestData::Hset(v)) => v.execute(store),
        Some(RequestData::Hmset(v)) => v.execute(store),
        Some(RequestData::Hdel(v)) => v.execute(store),
        Some(RequestData::Hmdel(v)) => v.execute(store),
        Some(RequestData::Hexist(v)) => v.execute(store),
        Some(RequestData::Hmexist(v)) => v.execute(store),
        None => KvError::InvalidCommand("Request has no data".into()).into(),
        _ => Value::default().into(),
    }
}

/// 从 Request 中得到 Response，目前处理所有 PUBLISH/SUBSCRIBE/UNSUBSCRIBE
pub fn dispatch_stream(cmd: CommandRequest, topic: impl Topic) -> StreamingResponse {
    match cmd.request_data {
        Some(RequestData::Publish(param)) => param.execute(topic),
        Some(RequestData::Subscribe(param)) => param.execute(topic),
        Some(RequestData::Unsubscribe(param)) => param.execute(topic),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemTable, Value};
    use http::StatusCode;
    use tokio_stream::StreamExt;
    use tracing::info;

    #[tokio::test]
    async fn service_should_work() {
        let service: Service = ServiceInner::new(MemTable::default()).into();
        // service 可以运行在多线程环境下，它的 clone 应该是轻量级的
        let cloned = service.clone();

        tokio::spawn(async move {
            let mut res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            let data = res.next().await.unwrap();
            assert_res_ok(&data, &[Value::default()], &[]);
        })
        .await
        .unwrap();

        let mut res = service.execute(CommandRequest::new_hget("t1", "k1"));
        let data = res.next().await.unwrap();
        assert_res_ok(&data, &["v1".into()], &[]);
    }

    #[tokio::test]
    async fn event_registration_should_work() {
        fn b(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Ok(())
        }

        fn c(resp: &CommandResponse) -> Result<(), KvError> {
            info!("Got {:?}", resp);
            Ok(())
        }

        fn d(res: &mut CommandResponse) -> Result<(), KvError> {
            res.status = StatusCode::CREATED.as_u16() as _;
            Ok(())
        }

        fn e() -> Result<(), KvError> {
            info!("Data is sent");
            Ok(())
        }

        let service: Service = ServiceInner::new(MemTable::default())
            .fn_received(|_: &CommandRequest| -> Result<(), KvError> { Ok(()) })
            .fn_received(b)
            .fn_executed(c)
            .fn_before_send(d)
            .fn_after_send(e)
            .into();

        let mut res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        let data = res.next().await.unwrap();
        assert_eq!(data.status, StatusCode::CREATED.as_u16() as u32);
        assert_eq!(data.message, "");
        assert_eq!(data.values, vec![Value::default()]);
    }

    #[tokio::test]
    async fn test_pipeline_should_return_early_when_something_went_wrong() {
        fn b(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Ok(())
        }

        fn c(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Err(KvError::Internal("Something went wrong".into()))
        }

        fn d(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Ok(())
        }

        let service: Service = ServiceInner::new(MemTable::default())
            .fn_received(|_: &CommandRequest| -> Result<(), KvError> { Ok(()) })
            .fn_received(b)
            .fn_received(c)
            .fn_received(d)
            .into();

        let mut res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        let data = res.next().await.unwrap();
        assert_res_error(
            &data,
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
            "Something went wrong",
        );
    }

    #[tokio::test]
    async fn test_pipeline_should_return_early_when_mut_fn_went_wrong() {
        fn b(cmd: &CommandRequest) -> Result<(), KvError> {
            info!("Got {:?}", cmd);
            Ok(())
        }

        fn c(resp: &CommandResponse) -> Result<(), KvError> {
            info!("Got {:?}", resp);
            Ok(())
        }

        fn d(res: &mut CommandResponse) -> Result<(), KvError> {
            res.status = StatusCode::CREATED.as_u16() as _;
            Ok(())
        }

        fn e(_: &mut CommandResponse) -> Result<(), KvError> {
            Err(KvError::Internal("Something went wrong".into()))
        }

        fn f(res: &mut CommandResponse) -> Result<(), KvError> {
            res.status = StatusCode::OK.as_u16() as _;
            Ok(())
        }

        let service: Service = ServiceInner::new(MemTable::default())
            .fn_received(|_: &CommandRequest| -> Result<(), KvError> { Ok(()) })
            .fn_received(b)
            .fn_executed(c)
            .fn_before_send(d)
            .fn_before_send(e)
            .fn_before_send(f)
            .into();

        let mut res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        let data = res.next().await.unwrap();
        assert_res_error(
            &data,
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
            "Something went wrong",
        );
    }
}

// 如果要在 command_service.rs 中调用 assert_res_ok，则 pub 是必要的
#[cfg(test)]
pub fn assert_res_ok(res: &CommandResponse, values: &[Value], pairs: &[Kvpair]) {
    let mut sorted_pairs = res.kvpairs.clone();
    sorted_pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "");
    assert_eq!(res.values, values);
    assert_eq!(sorted_pairs, pairs);
}

#[cfg(test)]
pub fn assert_res_error(res: &CommandResponse, code: u32, msg: &str) {
    assert_eq!(res.status, code);
    assert!(res.message.contains(msg));
    assert_eq!(res.values, &[]);
    assert_eq!(res.kvpairs, &[]);
}
