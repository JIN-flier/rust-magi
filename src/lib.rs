//! MAGI 的库入口：按身份创建 Agent，运行文章评审、会议或投票。
//!
//! 三种工作流均注入三个 Rig Agent；库本身不加载 `.env`，也不打印业务结果。
//! 配置文件读取及工具调用仍未实现；完整模块导航见 `src/README.md`。

/// 统一身份、模型工厂和 Agent 执行循环。
pub mod agent;
/// 基于 Tokio mpsc 的消息协议、收件箱和响应转发。
pub mod bus;
/// 三个 Agent 的模型名称配置。
pub mod config;
/// 默认模型名和任务提示词常量。
pub mod constance;
/// 根据 Agent 身份拼接系统提示词。
pub mod prompt;
/// 业务流程编排、结果汇总和后台任务生命周期管理。
pub mod workflow;
