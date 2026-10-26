//! CLI 示例的默认值。`constance` 和 `PREABLE` 均沿用既有命名。
//!
//! 三个模型名按 Balthasar、Casper、Melchior 的顺序传给配置对象。
//! 工厂目前只使用 `AGENT1_PREABLE`；另外两个提示词尚未接入模型构造。

/// Balthasar（作者）默认使用的模型名称；是否可用由所配置的服务端决定。
pub const AGENT1_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";
/// Casper（评审）默认使用的模型名称。
pub const AGENT2_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";
/// Melchior（事实核查）默认使用的模型名称。
pub const AGENT3_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";
/// 写作和评审的基础提示词；当前工厂为三个角色均使用此值。
pub const AGENT1_PREABLE: &str = "you are a helpful assistant. You can write articals about computer tech. If there are alreay an artical then you review it and give advice";
/// 为评审角色预留的基础提示词，目前未被工厂使用。
pub const AGENT2_PREABLE: &str =
    "you are a helpful assistant. You can review articals and give advice.";
/// 为事实核查角色预留的基础提示词，目前未被工厂使用。
pub const AGENT3_PREABLE: &str =
    "you are a helpful assistant. You can check if a artical's information is right.";

pub const CHEAP_MODEL_NAME: &str = "nvidia/nemotron-3.5-lightning:free";

/// 会议和投票共用的基础提示词；具体问题和输出 schema 由工作流提供。
pub const MAGI_GENERAL_PREABLE: &str = "You are a MAGI member. Analyze the user's question independently, explain your reasoning, and follow the workflow's requested output schema.";
