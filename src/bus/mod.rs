//! 与业务流程无关的通信模块；协议、总线和收件箱均定义在 [`crate::bus::message`] 中。
//!
//! 总线没有独立路由任务，需要所有者持续调用 [`crate::bus::message::MessageBus::recv`]。

/// 消息类型、中心响应路由和每个 Agent 的收件箱。
pub mod message;
