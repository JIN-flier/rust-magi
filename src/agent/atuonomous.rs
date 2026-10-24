//! 串行消费单个收件箱的 Agent 执行循环，供工作流放入 Tokio 任务运行。

use anyhow::{Context, Result, bail};
use rig::prelude::*;

use crate::bus::message::{AgentMailbox, AgentMessage};

/// 执行任务并记录自己及其他 Agent 的响应。
///
/// 历史仅保存在内存中，当前不会传给模型，也不会持久化或对外返回。
/// 每个实例串行处理消息；多个实例可由工作流并行运行。
pub struct AutonomousAgent {
    agent: Agent,
    mailbox: AgentMailbox,
    history: Vec<String>,
}

impl AutonomousAgent {
    /// 接管一个 Rig Agent 和专属收件箱；身份取自收件箱，此时不启动任务。
    pub fn new(agent: Agent, mailbox: AgentMailbox) -> Self {
        Self {
            agent,
            mailbox,
            history: Vec::new(),
        }
    }

    /// 持续处理消息，直到收到 Shutdown 或收件箱关闭。
    ///
    /// Task / Trigger 各触发一次模型请求，Response 只记录历史，不触发新请求。
    /// 模型请求期间不会处理后续消息，因此 Shutdown 不会抢占进行中的请求；
    /// 超时和取消由拥有后台任务的工作流处理。
    ///
    /// # Errors
    ///
    /// 模型请求失败、发布响应失败或收到 Error 消息时返回错误。
    /// 模型错误包含当前身份；正常关闭返回 `Ok(())`。
    pub async fn run(mut self) -> Result<()> {
        while let Some(message) = self.mailbox.recv().await {
            match message {
                AgentMessage::Task(prompt) | AgentMessage::Trigger(prompt) => {
                    let response = self.agent.prompt(prompt).await.with_context(|| {
                        format!("Agent {} model request failed", self.mailbox.id())
                    })?;
                    let content = response.output().to_string();
                    self.history.push(content.clone());
                    self.mailbox.respond(content).await?;
                }
                AgentMessage::Response(from, content) => {
                    self.history.push(format!("From {from}: {content}"));
                }
                AgentMessage::Shutdown => break,
                AgentMessage::Error(error) => bail!("Agent {}: {error}", self.mailbox.id()),
            }
        }
        Ok(())
    }
}
