use crate::*;

mod command_service;

/// 对 Command 的处理进行抽象
pub trait CommandService {
    /// 处理 Command 返回 Response
    fn execute(self, store: &impl Storage) -> CommandResponse;
}
