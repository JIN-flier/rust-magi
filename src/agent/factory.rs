//! 定义统一身份并按配置构造真实服务端的 Rig Agent。
//!
//! 离线测试可直接使用 Rig 的 Mock 模型，无需经过此工厂。

use rig::prelude::*;
use rig::providers::openai::OpenAI;

use crate::config::config::MagiConfiguration;
use crate::constance::AGENT1_PREABLE;
use crate::prompt::create_prompt::create_sys_prompt;

/// 全项目统一的 Agent 身份，用于配置查询、消息来源和路由键。
///
/// `Display` 输出大写名称；派生的排序只供容器使用，不表示业务执行顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MagiCoreID {
    /// 文章评审流程中的事实核查者。
    Melchior,
    /// 文章评审流程中的作者。
    Balthasar,
    /// 文章评审流程中的改进建议评审者。
    Casper,
    /// fast & cheap model
    FastCheap,
}

impl std::fmt::Display for MagiCoreID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Melchior => "MELCHIOR",
            Self::Balthasar => "BALTHASAR",
            Self::Casper => "CASPER",
            Self::FastCheap => "FastCheap",
        })
    }
}

/// 将统一身份与已构造的 Rig Agent 绑定，不负责消息路由或后台任务管理。
#[derive(Clone)]
pub struct MagiCore {
    id: MagiCoreID,
    agent: Agent,
    // history: ChatHistory,
    // memory: MemoryStore,
}

impl MagiCore {
    /// 从配置中选择该身份的模型名，拼接系统提示词并构造 Rig Agent。
    ///
    /// 通过 `OpenAI::from_env()` 读取 `OPENAI_API_KEY` 和可选的 `OPENAI_BASE_URL`。
    /// 此方法不加载 `.env`，也不发送 completion 请求；调用方负责环境准备。
    /// 当前三个身份均传入 `AGENT1_PREABLE`，仅人格前缀随身份变化。
    ///
    /// # Errors
    ///
    /// 服务端客户端无法从环境配置初始化时返回错误。
    pub fn new(id: MagiCoreID, magi_config: &MagiConfiguration) -> anyhow::Result<Self> {
        Self::new_with_preamble(id, magi_config, AGENT1_PREABLE)
    }

    /// 使用指定的基础系统提示词构造 MAGI；适用于非文章类工作流。
    /// 人格前缀仍由身份决定，模型名仍从配置中读取。
    pub fn new_with_preamble(
        id: MagiCoreID,
        magi_config: &MagiConfiguration,
        preamble: &str,
    ) -> anyhow::Result<Self> {
        let model_name = magi_config.get_model_name(&id);
        let system_prompt = create_sys_prompt(&id, preamble);
        let magi_core = MagiCore {
            id,
            agent: AgentBuilder::new(OpenAI::from_env()?.completion(model_name))
                .preamble(system_prompt)
                .build(),
        };
        Ok(magi_core)
    }
    /// 借用该核心绑定的身份。
    pub fn get_id(&self) -> &MagiCoreID {
        &self.id
    }
    /// 借用底层 Rig Agent；需要交给工作流时可克隆该句柄。
    pub fn get_agent(&self) -> &Agent {
        &self.agent
    }
    /// 预留的非流式对话接口，目前未实现。
    ///
    /// # Panics
    ///
    /// 当前调用必定触发 `todo!()`。现阶段应通过 Rig Agent 或工作流发起请求。
    pub async fn chat_completion(&mut self, msg: &str) -> Self {
        todo!()
    }
    /// 预留的流式对话接口，目前未实现。
    ///
    /// # Panics
    ///
    /// 当前调用必定触发 `todo!()`，尚不提供输出流。
    pub async fn stream_completion(&mut self, msg: &str) -> Self {
        todo!()
    }
}
