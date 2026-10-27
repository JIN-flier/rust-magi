# src 交接导航

当前 CLI 根据意图调用投票或会议工作流。文章评审工作流与 examples 已移除。根目录 [README](../README.md) 给出运行方法和后续 TODO。

| 入口或模块 | 当前职责 | 详细说明 |
| --- | --- | --- |
| [main.rs](main.rs) | 加载 `.env`、读取命令行问题、识别意图并运行对应工作流 | 下文 |
| [lib.rs](lib.rs) | 导出库模块；Rust 导入名为 `rust_magi` | 各模块文档 |
| [agent](agent/README.md) | 三位成员与 `FastCheap` 身份、模型工厂、意图识别、消息消费循环 | `MagiCore`、`IntentWorkflow`、`AutonomousAgent` |
| [bus](bus/README.md) | 有界收件箱、定向消息和公开响应转发 | `AgentMessage`、`MessageBus` |
| [config](config/README.md) | 四个身份的模型名称 | `MagiConfiguration` |
| [prompt](prompt/README.md) | 人格、意图、投票与会议提示词 | `create_prompt`、`intent_recognise` |
| [workflow](workflow/README.md) | 投票与会议编排及结果展示 | `VoteWorkflow`、`MeetingWorkflow`、`WorkflowResult` |
| [constance.rs](constance.rs) | CLI 默认模型名与基础提示词 | 下文 |

## CLI 数据流

```text
命令行参数拼成 query
  → MagiConfiguration::new（Balthasar、Casper、Melchior、FastCheap 的模型名）
  → IntentWorkflow::run(&query)
     ├─ Intention::Vote { option_a, option_b }
     │    → VoteWorkflow::run(query, &intention) → VoteResult
     └─ Intention::Meeting { interpretation }
          → MeetingWorkflow::run(query) → MeetingResult
  → WorkflowResult::show() → 标准输出
```

`main` 使用 `dotenvy::dotenv()?`，因此即使环境变量已在进程中设置，找不到 `.env` 仍会返回错误。`OpenAI::from_env()` 使用 `OPENAI_API_KEY`，兼容服务可设置 `OPENAI_BASE_URL`。空白问题会返回错误。意图识别、投票和会议都需要真实模型服务；库和离线测试不依赖该服务。当前会议分支只展示 `interpretation`，发给会议 Agent 的仍是原始 query。

```bash
cargo test --all-targets             # Mock 模型离线测试
cargo doc --no-deps                # 生成接口文档
cargo run -- "如何发布新版本？"   # 真实模型调用
```

## 默认值与边界

`AGENT1_MODEL_NAME`、`AGENT2_MODEL_NAME`、`AGENT3_MODEL_NAME` 分别给 Balthasar、Casper、Melchior；`CHEAP_MODEL_NAME` 给意图识别用的 `FastCheap`。`MAGI_GENERAL_PREABLE` 用于 CLI 中三位成员，工厂的 `MagiCore::new` 则仍使用 `AGENT1_PREABLE`。`AGENT2_PREABLE` 和 `AGENT3_PREABLE` 尚未接入。

- 配置文件搜索、创建和加载仍为 `todo!()`；目前使用 `MagiConfiguration::new`。
- `MagiCore::chat_completion` 与 `stream_completion` 尚未实现。
- `AutonomousAgent` 的响应历史只存于内存；当前投票与会议不使用 `MessageBus`。
- 会议默认最多 10 轮、3000 秒；投票默认 120 秒。超时可由 `with_timeout` 修改。
- `tests/meeting_workflow.rs`、`tests/vote_workflow.rs` 与 `tests/prompts.rs` 覆盖当前业务；`tests/bus.rs`、`tests/mock_agent.rs` 和 `tests/config.rs` 覆盖保留的基础模块。

后续工作顺序：从文件读取 config、实现 history 和 memory 管理、流式输入输出、更好的 UI。修改公共接口时，同步更新对应模块 README 与测试。
