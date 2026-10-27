# agent：身份、模型工厂与执行循环

[返回 src 导航](../README.md)

本模块包含三部分：`factory.rs` 定义身份并构造 Rig Agent；`intent.rs` 用单独模型识别投票或会议意图；`atuonomous.rs` 接管 Agent 和收件箱执行消息任务。当前投票与会议由 [workflow](../workflow/README.md) 直接编排，消息投递能力见 [bus](../bus/README.md)。

## factory 子模块

导入路径：`rust_magi::agent::factory::{MagiCoreID, MagiCore}`。

`MagiCoreID` 是全项目唯一的身份类型：

| 变体 | 当前用途 | Display 输出 |
| --- | --- | --- |
| `Balthasar` | 投票和会议成员 | `BALTHASAR` |
| `Casper` | 投票和会议成员 | `CASPER` |
| `Melchior` | 投票和会议成员 | `MELCHIOR` |
| `FastCheap` | 意图识别模型 | `FastCheap` |

枚举支持 Copy、相等比较和排序，可作为路由表键；枚举声明顺序不是流程执行顺序。

| MagiCore 接口 | 功能与返回值 | 状态/错误 |
| --- | --- | --- |
| `new(id, &MagiConfiguration) -> anyhow::Result<Self>` | 查模型名，拼接系统提示词，创建带身份的 Rig Agent | 环境初始化失败返回错误；此时不发送模型请求 |
| `new_with_preamble(id, &MagiConfiguration, preamble) -> anyhow::Result<Self>` | 使用指定基础提示词创建带身份的 Agent，供 CLI 中的会议和投票使用 | 同上 |
| `get_id(&self) -> &MagiCoreID` | 借用绑定身份 | 已实现 |
| `get_agent(&self) -> &Agent` | 借用 Rig Agent，调用方可克隆句柄交给工作流 | 已实现 |
| `chat_completion(&mut self, &str) -> Self`（async） | 预留非流式对话封装 | 未实现，调用会 panic |
| `stream_completion(&mut self, &str) -> Self`（async） | 预留流式对话封装，尚无输出流 | 未实现，调用会 panic |

工厂读取 `OPENAI_API_KEY` 和可选的 `OPENAI_BASE_URL`，不会主动加载 `.env`。配置中只有模型名，凭证由环境提供。当前 `new` 为三位成员传入 `AGENT1_PREABLE`，再由 [prompt](../prompt/README.md) 加上身份前缀；`AGENT2_PREABLE`、`AGENT3_PREABLE` 尚未接入。`MagiCore` 的系统提示词构造不支持 `FastCheap`，该身份由 `IntentWorkflow` 专用。

## intent 子模块

导入路径：`rust_magi::agent::intent::{IntentWorkflow, Intention}`。`IntentWorkflow::new(MagiCoreID::FastCheap, &config)` 使用对应模型名和内部意图提示词构造识别 Agent；`run(&query)` 返回 `Intention::Vote { option_a, option_b }` 或 `Intention::Meeting { interpretation }`。`with_retries(times)` 修改请求重试次数，默认 5。此构造函数不会检查传入身份是否为 `FastCheap`，CLI 固定使用该身份。

投票意图的两个选项由 `VoteWorkflow::run` 使用；会议意图中的 `interpretation` 目前只由 CLI 打印，会议工作流仍接收原始 query。意图识别需要真实模型服务，当前没有 Mock Agent 注入入口。

## atuonomous 子模块

导入路径：`rust_magi::agent::atuonomous::AutonomousAgent`。`atuonomous` 是现有文件及模块拼写。

| 接口 | 功能与所有权 |
| --- | --- |
| `new(agent: Agent, mailbox: AgentMailbox) -> Self` | 接管模型和专属收件箱，身份来自收件箱；不启动后台任务 |
| `run(self) -> anyhow::Result<()>`（async） | 串行消费消息直至退出，通常由 Tokio / JoinSet 执行 |

| 收到的消息 | 处理方式 |
| --- | --- |
| `Task(prompt)` / `Trigger(prompt)` | 发起模型请求，将文本记入历史，再通过 `respond` 发布 |
| `Response(from, content)` | 只记录 `From {from}: {content}`，不自动再次调用模型 |
| `Shutdown` | 正常结束循环，返回 `Ok(())` |
| `Error(reason)` | 返回包含身份和原因的错误 |
| 收件箱关闭且排空 | 正常退出 |

模型失败会作为 `run` 的错误返回，不会自动发送 `AgentMessage::Error`。发布失败也会结束任务。模型调用占用当前循环期间，Shutdown 只能排队等待；需要抢先取消时，由工作流中止 Tokio 任务。

历史仅为当前实例的内存记录，没有传给模型、持久化或公开读取接口。不要把当前行为理解为已实现多轮记忆。

## 使用方式

当前 CLI 直接使用 `IntentWorkflow`、`VoteWorkflow` 和 `MeetingWorkflow`。若单独使用 actor，需要同时驱动总线，否则公共响应不会转发：

```rust
use rig::{prelude::*, test_utils::MockCompletionModel};
use rust_magi::{
    agent::{atuonomous::AutonomousAgent, factory::MagiCoreID},
    bus::message::{AgentMessage, MessageBus},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let id = MagiCoreID::Balthasar;
    let mut bus = MessageBus::new(&[id], 8)?;
    let model = AgentBuilder::new(MockCompletionModel::text("示例文章")).build();
    let actor = AutonomousAgent::new(model, bus.take_mailbox(id)?);
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(actor.run());
    bus.send(id, AgentMessage::Task("写一篇文章".into())).await?;
    let response = bus.recv().await?;
    assert_eq!(response, AgentMessage::Response(id, "示例文章".into()));
    bus.shutdown().await?;
    while let Some(result) = tasks.join_next().await {
        result??;
    }
    Ok(())
}
```

这是正常路径示例。actor 不是当前投票或会议工作流的一部分；测试入口为 `cargo test --test mock_agent --test bus`，不使用真实服务。
