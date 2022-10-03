use crate::command_request::*;
use crate::*;
use std::sync::Arc;
use tracing::debug;
mod command_service;
mod topic;

/// 对 Command 的处理进行抽象
pub trait CommandService {
    /// 处理 Command 返回 Response
    fn execute(self, store: &impl Storage) -> CommandResponse;
}

/// Service 数据结构，其作用是将 CommandService 和 Storage 这两个 Trait 联合起来
/// 避免向用户暴露底层细节
pub struct Service<Store = MemTable> {
    inner: Arc<ServiceInner<Store>>,
}

impl<Store> Clone for Service<Store> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
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
    pub fn execute(&self, cmd: CommandRequest) -> CommandResponse {
        debug!("Got request: {:?}", cmd);
        if let Err(e) = self.inner.on_received.notify(&cmd) {
            return e;
        }
        let mut res = dispatch(cmd, &self.inner.store);
        debug!("Executed response: {:?}", res);
        if let Err(e) = self.inner.on_executed.notify(&res) {
            return e;
        }
        if let Err(e) = self.inner.on_before_send.notify(&mut res) {
            return e;
        }
        if !self.inner.on_after_send.is_empty() {
            debug!("Modified response: {:?}", res);
        }
        res
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemTable, Value};
    use http::StatusCode;
    use std::thread;
    use tracing::info;

    #[test]
    fn service_should_work() {
        let service: Service = ServiceInner::new(MemTable::default()).into();
        let cloned = service.clone();
        let handle = thread::spawn(move || {
            let res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            assert_res_ok(&res, &[Value::default()], &[]);
        });
        handle.join().unwrap();

        let res = service.execute(CommandRequest::new_hget("t1", "k1"));
        assert_res_ok(&res, &["v1".into()], &[]);
    }

    #[test]
    fn event_registration_should_work() {
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

        let res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        assert_eq!(res.status, StatusCode::CREATED.as_u16() as u32);
        assert_eq!(res.message, "");
        assert_eq!(res.values, vec![Value::default()]);
    }

    #[test]
    fn test_pipeline_should_return_early_when_something_went_wrong() {
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

        let res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        assert_res_error(
            &res,
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
            "Something went wrong",
        );
    }

    #[test]
    fn test_pipeline_should_return_early_when_mut_fn_went_wrong() {
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

        let res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        assert_res_error(
            &res,
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
