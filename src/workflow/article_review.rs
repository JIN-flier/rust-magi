use std::time::Duration;

use anyhow::{Context, Result, bail};
use rig::prelude::*;
use tokio::{task::JoinSet, time::timeout};

use crate::{
    agent::{atuonomous::AutonomousAgent, factory::MagiCoreID},
    bus::message::{AgentMessage, MessageBus},
};

const WRITER: MagiCoreID = MagiCoreID::Balthasar;
const REVIEWER: MagiCoreID = MagiCoreID::Casper;
const FACT_CHECKER: MagiCoreID = MagiCoreID::Melchior;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleReviewResult {
    pub article: String,
    pub review: String,
    pub fact_check: String,
}

/// Writes an article, then reviews and fact-checks it concurrently.
///
/// Agents are injected so callers can choose providers or offline mock models.
/// Each run owns fresh mailboxes and tasks, including when this value is reused.
pub struct ArticleReviewWorkflow {
    writer: Agent,
    reviewer: Agent,
    fact_checker: Agent,
    timeout: Duration,
}

impl ArticleReviewWorkflow {
    pub fn new(writer: Agent, reviewer: Agent, fact_checker: Agent) -> Self {
        Self {
            writer,
            reviewer,
            fact_checker,
            timeout: Duration::from_secs(120),
        }
    }

    /// Bound the entire run, including message delivery and graceful shutdown.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn run(&self, task: impl Into<String>) -> Result<ArticleReviewResult> {
        let mut bus = MessageBus::new(&[WRITER, REVIEWER, FACT_CHECKER], 64)?;
        // JoinSet also aborts its tasks if the caller drops this run future.
        let mut agents = JoinSet::new();
        for (id, agent) in [
            (WRITER, &self.writer),
            (REVIEWER, &self.reviewer),
            (FACT_CHECKER, &self.fact_checker),
        ] {
            let actor = AutonomousAgent::new(agent.clone(), bus.take_mailbox(id)?);
            agents.spawn(actor.run());
        }

        let mut result = timeout(self.timeout, async {
            let result = tokio::select! {
                biased;
                completed = agents.join_next() => {
                    completed.context("No agent tasks running")?
                        .context("Agent task failed")??;
                    bail!("Agent exited before workflow completed");
                }
                result = Self::exchange(&mut bus, task.into()) => result?,
            };
            bus.shutdown().await?;
            while let Some(completed) = agents.join_next().await {
                completed.context("Agent task failed")??;
            }
            Ok(result)
        })
        .await
        .context("Article review workflow timed out")
        .and_then(|result| result);

        // On failure, stop and join every remaining task. Preserve model/panic
        // errors if a disconnected mailbox was observed before the task result.
        agents.abort_all();
        while let Some(completed) = agents.join_next().await {
            match completed {
                Ok(Err(error)) => result = Err(error),
                Err(error) if !error.is_cancelled() => {
                    result = Err(error).context("Agent task failed");
                }
                _ => {}
            }
        }
        result
    }

    async fn exchange(bus: &mut MessageBus, task: String) -> Result<ArticleReviewResult> {
        bus.send(WRITER, AgentMessage::Task(task)).await?;
        let article = loop {
            match bus.recv().await? {
                AgentMessage::Response(from, content) if from == WRITER => break content,
                AgentMessage::Error(error) => bail!("{error}"),
                _ => {}
            }
        };

        bus.send(
            REVIEWER,
            AgentMessage::Trigger(format!(
                "请评审下面的技术文章，提出三条具体改进建议。\n\n文章内容：\n{article}"
            )),
        )
        .await?;
        bus.send(FACT_CHECKER, AgentMessage::Trigger(format!(
            "请检查下面的技术文章是否存在知识错误，如果有则提出具体建议。\n\n文章内容：\n{article}"
        ))).await?;

        let mut review = None;
        let mut fact_check = None;
        while review.is_none() || fact_check.is_none() {
            match bus.recv().await? {
                AgentMessage::Response(from, content) if from == REVIEWER => review = Some(content),
                AgentMessage::Response(from, content) if from == FACT_CHECKER => {
                    fact_check = Some(content)
                }
                AgentMessage::Error(error) => bail!("{error}"),
                _ => {}
            }
        }
        Ok(ArticleReviewResult {
            article,
            review: review.context("Reviewer produced no result")?,
            fact_check: fact_check.context("Fact checker produced no result")?,
        })
    }
}
