use rig::prelude::*;
use rig::providers::openai::OpenAI;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, interval};


/// Messages exchanged between agents.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    Task(String),
    Response(String, String), // (from_agent_id, content)
    Trigger(String),
    Shutdown,
    Error(String),
}