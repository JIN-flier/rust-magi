# workflow：文章评审、会议与投票

## 会议与投票

`MeetingWorkflow::new(balthasar, casper, melchior)` 接收三个 Rig Agent。`run(query)` 首轮把同一问题并行交给三人；后续每轮把此前三人的所有发言和提案、同一版候选结论放进提示词。模型通过 schema 返回 `message`（发言）、`proposal`（完整候选结论）和 `agrees_with_candidate`。首轮三人的提案完全相同时可直接形成共识；否则采用 Balthasar 的提案作为候选，有异议时采用首位异议者的提案作为下一轮候选。三人同轮明确认可候选时，`MeetingResult.consensus` 为 `Some`；达到 `with_max_turns` 设定的轮数（默认 5）则为 `None`，并保留 `rounds`。一轮是每位 Agent 各请求一次。默认整体超时 120 秒，可用 `with_timeout` 修改。

`VoteWorkflow::new(balthasar, casper, melchior)` 同样接收三个 Rig Agent。`run(query)` 同时向三人发送相同问题，使用 Rig `prompt_typed` 按 schema 收集 `choice: bool` 与 `reason: String`。`VoteResult` 保存三份带身份的票和 `decision: bool`；至少两票 `true` 时结果为 `true`。任一票无效、理由为空、请求失败或超时都返回错误，不生成决议。默认整体超时 120 秒，可用 `with_timeout` 修改。

两个新工作流会取消失败或超时后仍在执行的请求。会议交流通过工作流保存并注入完整讨论记录完成，不依赖当前总线；现有 `AutonomousAgent` 虽会记录收到的同伴响应，但不会把该历史自动传给模型。示例入口：`cargo run --example meeting -- "如何发布新版本？"` 与 `cargo run --example vote -- "是否发布新版本？"`。

## 文章评审

[返回 src 导航](../README.md)

实现位于 [article_review.rs](article_review.rs)，导入路径为 `rust_magi::workflow::article_review::{ArticleReviewWorkflow, ArticleReviewResult}`。它负责业务顺序和后台任务生命周期，模型调用由 [agent](../agent/README.md) 负责，通道与转发由 [bus](../bus/README.md) 负责。

## 公共接口

| 接口 | 功能与返回值 |
| --- | --- |
| `ArticleReviewWorkflow::new(writer: Agent, reviewer: Agent, fact_checker: Agent) -> Self` | 注入三个 Rig Agent，构造时不调用模型；默认超时 120 秒 |
| `with_timeout(self, timeout: Duration) -> Self` | 修改超时，支持链式调用 |
| `run(&self, task: impl Into<String>) -> anyhow::Result<ArticleReviewResult>`（async） | 将 task 发给作者，完成后并行评审，返回三份结果 |

参数位置决定角色：writer 对应 `MagiCoreID::Balthasar`，reviewer 对应 `Casper`，fact_checker 对应 `Melchior`。工作流接收的是 Rig Agent，而不是 MagiCore，因此不会验证传入模型的系统提示词是否与角色一致。

| ArticleReviewResult 字段 | 来源与含义 |
| --- | --- |
| `article: String` | Balthasar 生成的文章 |
| `review: String` | Casper 提供的改进建议 |
| `fact_check: String` | Melchior 提供的事实核查文本 |

工作流不打印输出，调用方决定展示方式。失败返回 Err，不返回部分结果。

## 执行顺序

1. 创建容量为 64 的总线及收件箱，用 JoinSet 启动三个 actor。
2. 向 Balthasar 发送 `Task(task)`，等待其 Response。总线同时把文章响应转发给另外两位 Agent。
3. 将文章分别嵌入两份 Trigger：要求 Casper 给出三条改进建议，要求 Melchior 检查知识错误并给出建议。
4. 两位 Agent 并行执行。工作流按发送者身份收集结果，不依赖响应到达顺序。
5. 三份结果齐全后，向全部 Agent 发送 Shutdown 并等待退出，再返回结果。

每次 `run` 都创建新的总线、actor 历史和任务集合。保存的 Rig Agent 会被克隆，底层模型或测试脚本是否共享状态由 Rig 决定；这不是服务端会话完全隔离的保证。

## 超时、失败与取消

- 模型错误由 actor 返回，包含 Agent 身份；工作流通过 JoinSet 监控任务，而不是等待一个不会到来的 Response。
- 监控到提前退出、panic 或路由失败时返回错误，并中止、等待剩余任务。清理时保留已完成任务的模型/panic 错误，以免被次生的通道关闭错误掩盖。
- `with_timeout` 覆盖业务执行、消息投递和正常 Shutdown 等待。超时后的取消与 join 清理在该窗口之外，不是严格的进程级硬截止时间。
- 调用方丢弃 `run` future 时，JoinSet 的 Drop 请求中止全部所属任务。Tokio 中止依赖协作式调度，不能抢占同步阻塞代码。

文章评审流程没有重试、持久化恢复、投票或共识判断。对等响应目前只在 actor 中记录，不自动成为模型上下文。

## 离线完整示例

```rust
use std::time::Duration;
use rig::{prelude::*, test_utils::MockCompletionModel};
use rust_magi::workflow::article_review::ArticleReviewWorkflow;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let writer = AgentBuilder::new(MockCompletionModel::text("Rust 所有权示例文章")).build();
    let reviewer = AgentBuilder::new(MockCompletionModel::text("建议增加借用示例")).build();
    let checker = AgentBuilder::new(MockCompletionModel::text("未发现知识错误")).build();
    let result = ArticleReviewWorkflow::new(writer, reviewer, checker)
        .with_timeout(Duration::from_secs(5))
        .run("介绍 Rust 所有权")
        .await?;
    assert_eq!(result.article, "Rust 所有权示例文章");
    assert_eq!(result.review, "建议增加借用示例");
    assert_eq!(result.fact_check, "未发现知识错误");
    Ok(())
}
```

该示例使用项目已启用的 Rig `test-utils`，无需 `.env` 或 API Key。接入真实模型时，可用 `MagiCore::new(...).get_agent().clone()` 构造对应参数，参考 [main.rs](../main.rs)。

## 测试与扩展位置

运行 `cargo test --test agent_flow`。现有测试覆盖结果及文章传递、双评审并发、各角色模型失败、panic、超时取消、调用方取消、失败后取消另一评审请求。

新增业务流程可新增 workflow 子模块并复用 bus / actor。若需要新的身份，先扩展 `MagiCoreID` 并检查配置与人格匹配；若需要新的消息语义，同步更新 `AgentMessage` 和 actor 匹配。不要把新流程的结束条件放进通用总线。
