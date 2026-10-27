//! 按身份选择人格前缀并拼接调用方提供的基础提示词。

use super::personality;
use crate::{
    agent::{factory::MagiCoreID, intent::Intention},
    workflow::meeting::MeetingRound,
};

/// 返回“人格前缀 + msg + 两个换行符”。
///
/// `msg` 原样拼接，不会去除空白、转义或进行模板替换；纯字符串操作，不调用模型。
/// 人格前缀来自私有 `personality` 模块，分别体现母亲、个体与科学家的判断视角。
pub fn create_sys_prompt(id: &MagiCoreID, msg: &str) -> String {
    let personality = match id {
        MagiCoreID::Balthasar => personality::BALTHASAR_PERSONALITY.to_string(),
        MagiCoreID::Casper => personality::CASPER_PERSONALITY.to_string(),
        MagiCoreID::Melchior => personality::MELCHIOR_PERSONALITY.to_string(),
        _ => todo!(), // this is for some future changes on config and MagiCoreID
    };
    personality + msg + "\n\n"
}

pub fn create_vote_prompt(vote_choise: &Intention, query: &str) -> String {
    let Intention::Vote { option_a, option_b } = vote_choise else {
        todo!();
    };
    format!(
        "用户问题：\n{query}\n\n请独立做出二元选择，并严格按 JSON schema 输出 choice（布尔值，选择`{option_a}`对应true，选择`{option_b}`对应false）和 reason（理由）。"
    )
}

pub fn create_meeting_prompt(
    query: &str,
    rounds: &[MeetingRound],
    candidate: Option<&str>,
) -> String {
    let mut prompt = format!(
        "用户问题：\n{query}\n\n请参与三人会议。以 JSON schema 输出 message、proposal 与 agrees_with_candidate。message 是公开发言；proposal 必须是能独立回答用户问题的完整结论。若反对当前候选，proposal 应给出可直接作为下一轮候选的替代结论。\n"
    );
    if let Some(candidate) = candidate {
        prompt.push_str(&format!("\n本轮共同评议的候选结论：\n{candidate}\n仅当你完全认可这一版候选结论时，agrees_with_candidate 才为 true，此时 proposal 必须原样复制候选结论。\n"));
    } else {
        prompt.push_str("\n首轮没有候选结论，agrees_with_candidate 必须为 false。\n");
    }
    if !rounds.is_empty() {
        prompt.push_str("\n此前公开讨论记录：\n");
        for (index, round) in rounds.iter().enumerate() {
            prompt.push_str(&format!("第 {} 轮：\n", index + 1));
            for message in &round.messages {
                prompt.push_str(&format!(
                    "{}：{}\n  提案：{}\n",
                    message.id, message.message, message.proposal
                ));
            }
        }
    }
    prompt
}

