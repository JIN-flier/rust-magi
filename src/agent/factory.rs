use rig::prelude::*;
use rig::providers::openai::OpenAI;

use crate::config::config::MagiConfiguration;
use crate::prompt::create_prompt::create_sys_prompt;
use crate::constance::{
    AGENT1_MODEL_NAME, AGENT1_PREABLE, AGENT2_MODEL_NAME, AGENT2_PREABLE, AGENT3_MODEL_NAME,
    AGENT3_PREABLE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MagiCoreID {
    Melchior,
    Balthasar,
    Casper,
}

#[derive(Clone)]
pub struct MagiCore {
    id: MagiCoreID,
    agent: Agent,
    // history: ChatHistory,
    // memory: MemoryStore,
}

impl MagiCore {
    pub fn new(
        id: MagiCoreID,
        magi_config: &MagiConfiguration,
    ) -> anyhow::Result<Self> {
        let model_name = magi_config.get_model_name(&id);
        let system_prompt = create_sys_prompt(&id, AGENT1_PREABLE);
        let magi_core = MagiCore {
            id: id,
            agent: AgentBuilder::new(OpenAI::from_env()?.completion(model_name))
                .preamble(system_prompt)
                .build(),
        };
        Ok(magi_core)
    }
    pub fn get_id(&self) -> &MagiCoreID {
        &self.id
    }
    pub fn get_agent(&self) -> &Agent {
        &self.agent
    }
    // sample chatcompletion with sync
    pub async fn chat_completion(&mut self, msg: &str) -> Self {
        todo!()
    }
    // stream chatcompletion
    pub async fn stream_completion(&mut self, msg: &str) -> Self {
        todo!()
    }
}
