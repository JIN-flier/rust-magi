//! 三位 MAGI 并行发言，并在下一轮阅读彼此发言，直到一致或达到轮数上限。

use std::time::Duration;

use anyhow::{Context, Result, ensure};
use rig::prelude::*;
use schemars::JsonSchema;
use serde::Deserialize;
use std::fmt::Write;
use tokio::time::timeout;

use crate::workflow::result::WorkflowResult;
use crate::{agent::factory::MagiCoreID, prompt::create_prompt::create_meeting_prompt};

const MAX_TIMEOUT: u64 = 3000;
const MAX_TURNS: usize = 10;

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
    /// Agent 参数依次绑定 Balthasar、Casper、Melchior；默认最多 5 轮、3000 秒。
    pub fn new(balthasar: Agent, casper: Agent, melchior: Agent) -> Self {
        Self {
            balthasar,
            casper,
            melchior,
            max_turns: MAX_TURNS,
            timeout: Duration::from_secs(MAX_TIMEOUT),
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
            let prompt = create_meeting_prompt(query, &rounds, candidate.as_deref());
            // try_join! 同时启动三次请求；某次失败时其余请求随 future 一起取消。
            let (balthasar, casper, melchior) = tokio::try_join!(
                self.speak(
                    &self.balthasar,
                    MagiCoreID::Balthasar,
                    &prompt,
                    candidate.as_deref()
                ),
                self.speak(
                    &self.casper,
                    MagiCoreID::Casper,
                    &prompt,
                    candidate.as_deref()
                ),
                self.speak(
                    &self.melchior,
                    MagiCoreID::Melchior,
                    &prompt,
                    candidate.as_deref()
                ),
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
}

impl WorkflowResult for MeetingResult {
    fn show(&self) -> String {
        let mut s = String::with_capacity(2000);
        for (index, round) in self.rounds.iter().enumerate() {
            writeln!(
                &mut s,
                "第 {} 轮，候选结论：{:?}",
                index + 1,
                round.candidate
            )
            .unwrap();
            for message in &round.messages {
                writeln!(
                    &mut s,
                    "{}：{}；提案：{}（认可候选：{}）",
                    message.id, message.message, message.proposal, message.agrees_with_candidate
                )
                .unwrap();
            }
        }
        match &self.consensus {
            Some(consensus) => writeln!(&mut s, "共同意见：{consensus}").unwrap(),
            None => writeln!(&mut s, "达到最大轮数，尚未取得三人共同意见").unwrap(),
        }
        s
    }
}
