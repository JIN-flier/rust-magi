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

/// 对议题的处理意图：判断这个议题下一步该怎么走。
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Intention {
    /// 议题收敛为二元选项时使用。
    Vote {
        /// 第一个选项的完整表述。
        option_a: String,
        /// 第二个选项的完整表述。必须与 option_a 互斥。
        option_b: String,
    },
    /// 议题需要会议讨论时使用。
    Meeting {
        /// 补出query背后没说清的东西。
        interpretation: String,
    },
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
            .prompt_typed::<Intention>(query.to_string())
            .max_turns(self.retry_times)
            .retries(self.retry_times)
            .await?;
        Ok(response.output)
    }
}

use std::fmt;
impl fmt::Display for Intention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Intention::Vote { option_a, option_b } => {
                write!(f, "投票：选项1[{}]，选项2[{}]", option_a, option_b)
            }
            Intention::Meeting{ interpretation } => {
                write!(f, "会议：{}", interpretation)
            }
        }
    }
}
