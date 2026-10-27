# workflow：投票、会议与结果展示

[返回 src 导航](../README.md)

模块包含 [vote.rs](vote.rs)、[meeting.rs](meeting.rs) 和 [result.rs](result.rs)。两个工作流接收按 Balthasar、Casper、Melchior 顺序传入的三个 Rig `Agent`，自身不加载环境、不打印输出，也不使用 `MessageBus`。输入提示词由 [prompt](../prompt/README.md) 中的函数生成。

## 投票 `VoteWorkflow`

```text
VoteWorkflow::new(balthasar, casper, melchior)
    .with_timeout(Duration::from_secs(120))
    .run(query, &Intention::Vote { option_a, option_b })
    -> anyhow::Result<VoteResult>
```

`run` 并发向三人发送问题和同一组二元选项。`option_a` 对应 `choice: true`，`option_b` 对应 `choice: false`；模型还须返回非空 `reason`。`VoteResult` 含按构造参数顺序排列的 `votes: Vec<Vote>` 和 `decision: bool`，至少两票 `true` 时为 `true`。任何一票解析失败、理由为空、模型错误或超时都返回错误，不生成部分决议。默认整体超时 120 秒。传入 `Intention::Meeting` 不受支持；调用方应先通过意图识别选择正确工作流。

## 会议 `MeetingWorkflow`

```text
MeetingWorkflow::new(balthasar, casper, melchior)
    .with_max_turns(10)
    .with_timeout(Duration::from_secs(3000))
    .run(query)
    -> anyhow::Result<MeetingResult>
```

每轮三人同时返回 `message`（公开发言）、`proposal`（可独立回答问题的完整结论）和 `agrees_with_candidate`。首轮没有候选，三人提案完全相同时立即达成一致；否则以 Balthasar 的提案作为候选。后续轮次把此前所有发言、提案和共同候选放进三人的提示词；若有人反对，采用按 Balthasar、Casper、Melchior 顺序遇到的第一份异议提案作为下一轮候选。三人同轮明确认可当前候选时返回共识。认可却改变候选文本、首轮无候选却声称认可、发言或提案为空，都会返回错误。

`MeetingResult` 包含 `consensus: Option<String>` 和完整 `rounds: Vec<MeetingRound>`；每轮记录当轮候选及三份 `MeetingMessage`。轮数耗尽且未一致时返回 `Ok`，其中 `consensus` 为 `None`。`with_max_turns(0)` 会在 `run` 时返回错误。一轮代表每位 Agent 各请求一次。默认最多 10 轮、整体超时 3000 秒。

两个工作流在模型错误或超时时取消其余尚在执行的请求。调用方也可直接丢弃运行中的 future 来取消。

## 结果展示与离线示例

`VoteResult` 与 `MeetingResult` 均实现 `WorkflowResult::show() -> String`。投票输出逐票理由及多数决；会议输出轮次、发言、提案和共同意见或未达成一致的提示。CLI 调用该 trait 打印结果。

```rust
use rig::{prelude::*, test_utils::MockCompletionModel};
use rust_magi::{
    agent::intent::Intention,
    workflow::{result::WorkflowResult, vote::VoteWorkflow},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let reply = r#"{"choice":true,"reason":"风险可控"}"#;
    let make_agent = || AgentBuilder::new(MockCompletionModel::text(reply)).build();
    let intent = Intention::Vote {
        option_a: "发布".into(),
        option_b: "延后".into(),
    };
    let result = VoteWorkflow::new(make_agent(), make_agent(), make_agent())
        .run("是否发布？", &intent)
        .await?;
    assert!(result.decision);
    println!("{}", result.show());
    Ok(())
}
```

示例使用 Rig Mock 模型，不需要 `.env`。真实模型可通过 `MagiCore::new_with_preamble(...).get_agent().clone()` 构造。验证命令：`cargo test --test vote_workflow --test meeting_workflow`。
