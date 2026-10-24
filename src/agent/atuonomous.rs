use anyhow::{Context, Result, bail};
use rig::prelude::*;

use crate::bus::message::{AgentMailbox, AgentMessage};

/// Executes model requests and records public responses from its peers.
pub struct AutonomousAgent {
    agent: Agent,
    mailbox: AgentMailbox,
    history: Vec<String>,
}

impl AutonomousAgent {
    pub fn new(agent: Agent, mailbox: AgentMailbox) -> Self {
        Self {
            agent,
            mailbox,
            history: Vec::new(),
        }
    }

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
