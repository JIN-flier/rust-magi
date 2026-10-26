//! 三位 MAGI 并行发言，并在下一轮阅读彼此发言，直到一致或达到轮数上限。

use std::time::Duration;

use anyhow::{Context, Result, ensure};
use rig::prelude::*;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time::timeout;

use crate::agent::factory::MagiCoreID;

const IDS: [MagiCoreID; 3] = [
    MagiCoreID::Balthasar,
    MagiCoreID::Casper,
    MagiCoreID::Melchior,
];

/// 一次会议发言、完整提案及其对当前候选结论的态度。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingMessage {
    pub id: MagiCoreID,
    pub message: String,
    pub proposal: String,
    pub agrees_with_candidate: bool,
}

/// 本轮供三位成员评议的候选结论和各自发言。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingRound {
    pub candidate: Option<String>,
    pub messages: Vec<MeetingMessage>,
}

/// `consensus` 为 None 表示轮数用尽仍未取得三人一致；保留完整讨论记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingResult {
    pub consensus: Option<String>,
    pub rounds: Vec<MeetingRound>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MeetingOutput {
    /// 对其他成员公开的发言，可解释自己的立场。
    message: String,
    /// 可独立回答用户问题的完整结论；认可当前候选时原样复制该候选。
    proposal: String,
    /// 仅当完全认可提示中的候选结论时为 true；没有候选结论时必须为 false。
    agrees_with_candidate: bool,
}

/// 每轮同时请求三个 Agent；下一轮把完整讨论记录和同一个候选结论交给三者。
pub struct MeetingWorkflow {
    balthasar: Agent,
    casper: Agent,
    melchior: Agent,
    max_turns: usize,
    timeout: Duration,
}

impl MeetingWorkflow {
    /// Agent 参数依次绑定 Balthasar、Casper、Melchior；默认最多 5 轮、120 秒。
    pub fn new(balthasar: Agent, casper: Agent, melchior: Agent) -> Self {
        Self {
            balthasar,
            casper,
            melchior,
            max_turns: 5,
            timeout: Duration::from_secs(120),
        }
    }

    /// 一轮包含三位成员各一次模型请求；0 轮会在 run 时返回错误。
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 首轮提案完全相同，或后续三人明确同意同一候选结论时返回共识。
    /// 模型错误或超时返回错误。
    pub async fn run(&self, query: impl Into<String>) -> Result<MeetingResult> {
        ensure!(self.max_turns > 0, "Meeting max_turns must be positive");
        let query = query.into();
        timeout(self.timeout, self.discuss(&query))
            .await
            .context("Meeting workflow timed out")?
    }

    async fn discuss(&self, query: &str) -> Result<MeetingResult> {
        let mut rounds = Vec::with_capacity(self.max_turns);
        let mut candidate: Option<String> = None;

        for _ in 0..self.max_turns {
            let prompt = Self::round_prompt(query, &rounds, candidate.as_deref());
            // try_join! 同时启动三次请求；某次失败时其余请求随 future 一起取消。
            let (balthasar, casper, melchior) = tokio::try_join!(
                self.speak(&self.balthasar, IDS[0], &prompt, candidate.as_deref()),
                self.speak(&self.casper, IDS[1], &prompt, candidate.as_deref()),
                self.speak(&self.melchior, IDS[2], &prompt, candidate.as_deref()),
            )?;
            let messages = vec![balthasar, casper, melchior];
            let unanimous = if candidate.is_some() {
                messages.iter().all(|m| m.agrees_with_candidate)
            } else {
                messages.iter().all(|m| m.proposal == messages[0].proposal)
            };
            let next_candidate = match &candidate {
                None => messages[0].proposal.clone(),
                Some(current) => messages
                    .iter()
                    .find(|m| !m.agrees_with_candidate)
                    .map(|m| m.proposal.clone())
                    .unwrap_or_else(|| current.clone()),
            };
            rounds.push(MeetingRound {
                candidate: candidate.clone(),
                messages,
            });
            if unanimous {
                return Ok(MeetingResult {
                    consensus: Some(next_candidate),
                    rounds,
                });
            }
            candidate = Some(next_candidate);
        }
        Ok(MeetingResult {
            consensus: None,
            rounds,
        })
    }

    async fn speak(
        &self,
        agent: &Agent,
        id: MagiCoreID,
        prompt: &str,
        candidate: Option<&str>,
    ) -> Result<MeetingMessage> {
        let output = agent
            .prompt_typed::<MeetingOutput>(prompt.to_owned())
            .await
            .with_context(|| format!("Agent {id} meeting response failed"))?
            .output;
        ensure!(
            !output.message.trim().is_empty(),
            "Agent {id} gave an empty meeting message"
        );
        ensure!(
            !output.proposal.trim().is_empty(),
            "Agent {id} gave an empty meeting proposal"
        );
        match candidate {
            None => ensure!(
                !output.agrees_with_candidate,
                "Agent {id} agreed without a meeting candidate"
            ),
            Some(current) if output.agrees_with_candidate => ensure!(
                output.proposal.trim() == current,
                "Agent {id} agreed but changed the meeting candidate"
            ),
            _ => {}
        }
        Ok(MeetingMessage {
            id,
            message: output.message.trim().to_owned(),
            proposal: output.proposal.trim().to_owned(),
            agrees_with_candidate: output.agrees_with_candidate,
        })
    }

    fn round_prompt(query: &str, rounds: &[MeetingRound], candidate: Option<&str>) -> String {
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
}
