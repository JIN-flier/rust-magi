//! 三位 MAGI 独立提交结构化二元选择与原因，然后计算多数票。

use std::time::Duration;

use anyhow::{Context, Result, ensure};
use rig::prelude::*;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time::timeout;

use crate::workflow::result::WorkflowResult;
use crate::{
    agent::{factory::MagiCoreID, intent::Intention},
    prompt::create_prompt::create_vote_prompt,
};
use std::fmt::Write; 

/// 一个 MAGI 的最终票。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vote {
    pub id: MagiCoreID,
    pub choice: bool,
    pub reason: String,
}

/// 三票齐全后的结果；票按 Balthasar、Casper、Melchior 排列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteResult {
    pub decision: bool,
    pub votes: Vec<Vote>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VoteOutput {
    /// 对用户二元问题的最终选择。
    choice: bool,
    /// 做出此选择的理由，不能留空。
    reason: String,
}

pub struct VoteWorkflow {
    balthasar: Agent,
    casper: Agent,
    melchior: Agent,
    timeout: Duration,
}

impl VoteWorkflow {
    /// Agent 参数依次绑定 Balthasar、Casper、Melchior；默认超时 120 秒。
    pub fn new(balthasar: Agent, casper: Agent, melchior: Agent) -> Self {
        Self {
            balthasar,
            casper,
            melchior,
            timeout: Duration::from_secs(120),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 三次模型请求并行执行。任何一票无效或失败均返回错误，不产生决议。
    pub async fn run(
        &self,
        query: impl Into<String>,
        vote_choise: &Intention,
    ) -> Result<VoteResult> {
        let query = query.into();
        timeout(self.timeout, async {
            let (balthasar, casper, melchior) = tokio::try_join!(
                self.cast(&self.balthasar, MagiCoreID::Balthasar, &query, vote_choise),
                self.cast(&self.casper, MagiCoreID::Casper, &query, vote_choise),
                self.cast(&self.melchior, MagiCoreID::Melchior, &query, vote_choise),
            )?;
            let votes = vec![balthasar, casper, melchior];
            let decision = votes.iter().filter(|vote| vote.choice).count() >= 2;
            Ok(VoteResult { decision, votes })
        })
        .await
        .context("Vote workflow timed out")?
    }

    async fn cast(
        &self,
        agent: &Agent,
        id: MagiCoreID,
        query: &str,
        vote_choise: &Intention,
    ) -> Result<Vote> {
        let prompt = create_vote_prompt(vote_choise, query);
        let output = agent
            .prompt_typed::<VoteOutput>(prompt)
            .await
            .with_context(|| format!("Agent {id} vote failed"))?
            .output;
        ensure!(
            !output.reason.trim().is_empty(),
            "Agent {id} gave an empty vote reason"
        );
        Ok(Vote {
            id,
            choice: output.choice,
            reason: output.reason.trim().to_owned(),
        })
    }
}

impl WorkflowResult for VoteResult {
    fn show(&self) -> String {
        let mut s = String::with_capacity(200);
        for vote in &self.votes {
            writeln!(&mut s, "{}：{}，理由：{}", vote.id, vote.choice, vote.reason).unwrap();
        }
        writeln!(&mut s, "多数决：{}", self.decision).unwrap();
        s
    }
}
