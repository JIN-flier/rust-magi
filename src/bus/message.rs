use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use tokio::sync::mpsc;

use crate::agent::factory::MagiCoreID;

/// Messages exchanged between agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMessage {
    Task(String),                 // send the first task message or prompt from user
    Response(MagiCoreID, String), // (from_agent_id, content)
    Trigger(String),              // trigger a task message or prompt from other agent
    Shutdown,
    Error(String), // contain error messages
}

/// Bounded mailboxes and a central response router, independent of any workflow.
///
/// The owner must drive `recv` to forward responses. Slow consumers apply
/// backpressure; the workflow is responsible for its overall timeout.
pub struct MessageBus {
    senders: BTreeMap<MagiCoreID, mpsc::Sender<AgentMessage>>,
    mailboxes: BTreeMap<MagiCoreID, AgentMailbox>,
    events: mpsc::Receiver<AgentMessage>,
}

/// One agent's inbox and its publishing connection to the bus.
pub struct AgentMailbox {
    id: MagiCoreID,
    inbox: mpsc::Receiver<AgentMessage>,
    events: mpsc::Sender<AgentMessage>,
}

impl MessageBus {
    pub fn new(agent_ids: &[MagiCoreID], capacity: usize) -> Result<Self> {
        ensure!(capacity > 0, "Message bus capacity must be positive");
        ensure!(
            !agent_ids.is_empty(),
            "Message bus needs at least one agent"
        );
        // bus:events <- mailbox:events
        let (events_tx, events) = mpsc::channel(capacity);
        let mut senders = BTreeMap::new();
        let mut mailboxes = BTreeMap::new();
        for &id in agent_ids {
            ensure!(!senders.contains_key(&id), "Duplicate agent ID: {id}");
            // bus:senders -> mailbox:inbox
            let (sender, inbox) = mpsc::channel(capacity); 
            senders.insert(id, sender);
            mailboxes.insert(
                id,
                AgentMailbox {
                    id,
                    inbox,
                    events: events_tx.clone(),
                },
            );
        }
        Ok(Self {
            senders,
            mailboxes,
            events,
        })
    }

    /// Each inbox can be handed to its agent only once.
    pub fn take_mailbox(&mut self, id: MagiCoreID) -> Result<AgentMailbox> {
        self.mailboxes
            .remove(&id)
            .with_context(|| format!("Unknown or already taken agent mailbox: {id}"))
    }

    pub async fn send(&self, id: MagiCoreID, message: AgentMessage) -> Result<()> {
        self.senders
            .get(&id)
            .with_context(|| format!("Unknown agent: {id}"))?
            .send(message)
            .await
            .with_context(|| format!("Agent {id} inbox closed"))
    }

    /// Receive a public response and forward it to every other agent.
    pub async fn recv(&mut self) -> Result<AgentMessage> {
        // get messgae
        let message = self
            .events
            .recv()
            .await
            .context("Bus event channel closed")?;
        // send message to all the other agents
        if let AgentMessage::Response(from, _) = &message {
            for id in self.senders.keys().filter(|id| *id != from) {
                self.send(*id, message.clone()).await?;
            }
        }
        Ok(message)
    }

    /// Try every inbox, even when an earlier agent has already disconnected.
    pub async fn shutdown(&self) -> Result<()> {
        let mut first_error = None;
        for id in self.senders.keys() {
            if let Err(error) = self.send(*id, AgentMessage::Shutdown).await {
                first_error.get_or_insert(error);
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl AgentMailbox {
    pub fn id(&self) -> MagiCoreID {
        self.id
    }

    pub async fn recv(&mut self) -> Option<AgentMessage> {
        self.inbox.recv().await
    }

    /// The sender ID is bound to the mailbox, so callers cannot spoof a peer.
    pub async fn respond(&self, content: String) -> Result<()> {
        self.events
            .send(AgentMessage::Response(self.id, content))
            .await
            .context("Bus event channel closed")
    }
}
