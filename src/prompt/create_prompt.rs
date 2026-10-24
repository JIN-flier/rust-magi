//! 按身份选择人格前缀并拼接调用方提供的基础提示词。

use super::personality;
use crate::agent::factory::MagiCoreID;

/// 返回“人格前缀 + msg + 两个换行符”。
///
/// `msg` 原样拼接，不会去除空白、转义或进行模板替换；纯字符串操作，不调用模型。
/// 人格前缀来自私有 `personality` 模块，当前仍为示例占位文本。
pub fn create_sys_prompt(id: &MagiCoreID, msg: &str) -> String {
    let personality = match id {
        MagiCoreID::Balthasar => personality::BALTHASAR_PERSONALITY.to_string(),
        MagiCoreID::Casper => personality::CASPER_PERSONALITY.to_string(),
        MagiCoreID::Melchior => personality::MELCHIOR_PERSONALITY.to_string(),
        // _ => personality::DEFAULT_PERSONALITY.to_string(),
    };
    personality + msg + "\n\n"
}
