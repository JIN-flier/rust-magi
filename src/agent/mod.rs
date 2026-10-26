//! Agent 层：`factory` 负责身份和模型构造，`atuonomous` 负责消息驱动的运行循环。
//!
//! `atuonomous` 沿用已有模块拼写；所有身份均使用 [`crate::agent::factory::MagiCoreID`]。

/// 消费收件箱并执行模型任务。
pub mod atuonomous;
/// 创建带身份和系统提示词的 Rig Agent。
pub mod factory;
/// 用于区分用户query的意图识别agent。
pub mod intent;