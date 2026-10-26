use crate::agent::factory::MagiCoreID;
use crate::config::config::MagiConfiguration;
use crate::prompt::intent_recognise::INTENT_PROMPT;
use rig::prelude::*;
use rig::providers::openai::OpenAI;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const INTENT_MAX_RETRY: usize = 5;

#[derive(Clone)]
pub struct IntentWorkflow {
    agent: Agent,
    retry_times: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct IntentionOutput {
    /// 意图识别结果：投票或会议讨论（true对应投票，False对应会议讨论）
    vote: bool,
    /// 投票或会议的原因
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intention {
    Vote(String),
    Meeting(String),
}

impl IntentWorkflow {
    pub fn new(id: MagiCoreID, magi_config: &MagiConfiguration) -> anyhow::Result<Self> {
        let model_name = magi_config.get_model_name(&id);
        let model = OpenAI::from_env()?.completion(model_name);
        let agent = AgentBuilder::new(model)
            .preamble(INTENT_PROMPT.to_string())
            .default_max_turns(INTENT_MAX_RETRY)
            .build();
        let retry_times = INTENT_MAX_RETRY;
        Ok(IntentWorkflow { agent, retry_times })
    }

    pub fn with_retries(mut self, times: usize) -> Self {
        self.retry_times = times;
        self
    }

    pub async fn run(&self, query: &str) -> anyhow::Result<Intention> {
        let response = self
            .agent
            .prompt_typed::<IntentionOutput>(query.to_string())
            .max_turns(self.retry_times)
            .retries(self.retry_times)
            .await?;
        if response.output.vote {
            Ok(Intention::Vote(response.output.reason))
        } else {
            Ok(Intention::Meeting(response.output.reason))
        }
    }
}

use std::fmt;
impl fmt::Display for Intention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Intention::Vote(reason) => write!(f, "投票：{}", reason),
            Intention::Meeting(reason) => write!(f, "会议：{}", reason),
        }
    }
}
