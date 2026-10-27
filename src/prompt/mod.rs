//! 将身份对应的人格前缀与任务提示词组合成系统提示词。
//!
//! 人格模板留在私有子模块中，外部调用方使用 [`crate::prompt::create_prompt::create_sys_prompt`]。

/// 系统提示词的公共拼接接口。
pub mod create_prompt;
/// 基于 EVA 中 MAGI 三种人格侧面的内部提示词。
mod personality;
/// 用于进行意图识别
pub mod intent_recognise;
