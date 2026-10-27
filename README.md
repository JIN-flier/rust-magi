# MAGI

MAGI 是一个用 Rust 和 [Rig](https://rig.rs/) 编写的三 Agent 协商 CLI，灵感来自《新世纪福音战士》中的 MAGI 系统。Balthasar、Casper 和 Melchior 使用不同的人格提示词，分别关注决定对人的影响、个人需求，以及事实和可行性。项目希望让这些视角针对同一个问题给出判断，并保留各自的理由与讨论过程。

当前第一阶段已经实现意图识别、二元投票和三人会议。用户在命令行输入问题后，程序先判断该用投票还是会议处理：投票收集三人的选择与理由，按多数票给出结果；会议让三人逐轮阅读彼此的发言和提案，直到认可同一候选结论，或达到轮数上限。现阶段的入口仍是基础 CLI；配置文件读取、history 和 memory 管理、流式输入输出及更好的 UI 列在后续计划中。详细接口见 [源码导航](src/README.md)。

## 当前流程

```text
命令行问题 → IntentWorkflow 识别 Vote / Meeting
             ├─ Vote：三人并发投票 → 多数决
             └─ Meeting：三人逐轮并发发言 → 一致结论或轮数耗尽
```

- `IntentWorkflow` 使用单独的 `FastCheap` 模型返回 `Intention::Vote { option_a, option_b }` 或 `Intention::Meeting { interpretation }`。投票的两个选项会传给三位成员；会议目前使用原始问题，`interpretation` 只在 CLI 中显示。
- `VoteWorkflow::run(query, &intention)` 要求投票意图，收集三份 `choice: bool` 与非空 `reason`。选项 A 对应 `true`，选项 B 对应 `false`；至少两票 `true` 时决议为 `true`。任一请求失败或回复无效则返回错误。
- `MeetingWorkflow::run(query)` 收集每轮三人的 `message`、完整 `proposal` 和对同一候选结论的 `agrees_with_candidate`。首轮提案相同或后续三人同轮认可候选时返回共识；达到轮数上限时返回 `consensus: None`，保留所有轮次。
- 两个结果都实现 `WorkflowResult::show()`，供 CLI 输出。会议默认上限 10 轮、整体超时 3000 秒；投票默认超时 120 秒。两者均可用 `with_timeout` 调整。

`bus` 和 `AutonomousAgent` 仍提供有界收件箱、公开响应转发和消息消费循环，但当前投票与会议工作流直接通过 Rig 并发请求，不使用总线。Agent 收到的历史目前只存于实例内存，没有进入模型上下文或持久化。文章评审工作流及旧 examples 已移除。

## 运行

需要 Rust 2024 edition 工具链、可用的 OpenAI 兼容模型服务，以及仓库根目录的 `.env`。CLI 会调用 `dotenvy::dotenv()?`，缺少 `.env` 会报错。至少设置 `OPENAI_API_KEY`；使用兼容服务时按需设置 `OPENAI_BASE_URL`。模型名称在 [constance.rs](src/constance.rs) 中指定，需由实际服务支持。不要提交凭证。

```bash
cargo run -- "是否应该发布这个版本？"
cargo run -- "如何设计发布流程？"
cargo test --all-targets
cargo doc --no-deps
```

库调用方可以直接构造 `MagiConfiguration`、Rig Agent 和相应工作流；库本身不加载 `.env`。离线测试使用 Rig Mock 模型，不需要 API Key。

## 后续 TODO

1. 从文件读取 config，包括路径、格式、默认值和错误处理。目前 `get_config_from_file` 尚未实现。
2. 实现 history 和 memory 管理，让跨轮上下文及持久化策略有明确接口。
3. 支持流式输入输出；`MagiCore::stream_completion` 目前仍是占位接口。
4. 提供更好的 UI，替代当前基础 CLI 输出。

模块细节：[agent](src/agent/README.md) · [bus](src/bus/README.md) · [config](src/config/README.md) · [prompt](src/prompt/README.md) · [workflow](src/workflow/README.md)。
