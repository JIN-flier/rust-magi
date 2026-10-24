# src 交接导航

当前可运行的业务是：Balthasar 生成文章，Casper 和 Melchior 并行给出改进建议与事实核查，最后返回三份文本。仓库根目录 README 描述的投票、讨论共识、工具网关等仍属于规划。

## 从哪里开始阅读

| 入口或模块 | 职责 | 接口说明 |
| --- | --- | --- |
| [main.rs](main.rs) | CLI 示例：加载环境、构造模型、运行流程、打印结果 | 本文“CLI 入口” |
| [lib.rs](lib.rs) | 导出库模块；Rust 导入名为 `rust_magi` | 本表中的模块文档 |
| [agent](agent/README.md) | 统一身份、模型工厂、消息消费循环 | `MagiCoreID`、`MagiCore`、`AutonomousAgent` |
| [bus](bus/README.md) | 收件箱、定向消息与公开响应转发 | `AgentMessage`、`MessageBus`、`AgentMailbox` |
| [config](config/README.md) | 保存每个身份的模型名称 | `MagiConfiguration` |
| [prompt](prompt/README.md) | 拼接人格前缀与基础系统提示词 | `create_sys_prompt`，以及私有人格模板说明 |
| [workflow](workflow/README.md) | 编排业务和管理后台任务 | `ArticleReviewWorkflow`、`ArticleReviewResult` |
| [constance.rs](constance.rs) | CLI 示例默认模型名、基础提示词 | 本文“默认常量” |

建议按 `main → workflow → agent → bus` 理解执行过程，再查看 `config` 和 `prompt`。每个目录 README 同时介绍其文件子模块，不需要按文件名再创建目录。

## 调用与数据流

```text
main
  ├─ MagiConfiguration::new：模型名
  ├─ MagiCore::new：身份 + 模型名 + create_sys_prompt → Rig Agent
  └─ ArticleReviewWorkflow::run
       ├─ MessageBus::new / take_mailbox：创建并分配收件箱
       ├─ JoinSet：启动三个 AutonomousAgent::run
       ├─ Task → Balthasar → Response(文章)
       ├─ Trigger(文章) → Casper / Melchior → 两份 Response
       └─ Shutdown + 等待任务退出 → ArticleReviewResult → main 打印
```

`AgentMailbox::respond` 只把响应放入公共队列；工作流调用 `MessageBus::recv` 后，总线才转发给其他 Agent 并将响应交给工作流。公共响应记录在 Agent 内存中，目前不会自动加入后续模型请求。

所有身份使用 `agent::factory::MagiCoreID`。模型名、提示词和输出内容仍是字符串。角色映射为 Balthasar＝作者、Casper＝改进建议评审、Melchior＝事实核查；不要使用整数或字符串代替身份。

## CLI 入口：main.rs

`main() -> anyhow::Result<()>` 在 Tokio 运行时中执行一次固定的 Rust 所有权写作任务，不解析命令行参数。库模块不加载环境、不打印业务结果，这些职责留在入口。

实际调用依赖 `.env` 和模型服务配置：

- `dotenvy::dotenv()?` 会查找并加载 `.env`；当前即使进程已经有环境变量，找不到 `.env` 仍会返回错误。
- `OpenAI::from_env()` 使用 `OPENAI_API_KEY`，并支持可选的 `OPENAI_BASE_URL`。根据实际服务填写；不要把真实凭证提交到仓库。
- 配置的模型名称必须由该服务支持。代码里的默认名称不保证在任意服务端均可用。

所有验证命令均从仓库根目录执行：

```bash
cargo test                         # 离线测试，无需 API Key
cargo doc --no-deps                # 生成接口文档到 target/doc
cargo clippy --all-targets         # 静态检查，目前有既有警告
cargo run                          # 真实模型调用，需要环境和服务配置
```

## 默认常量：constance.rs

| 常量 | 用途与当前状态 |
| --- | --- |
| `AGENT1_MODEL_NAME` | CLI 中 Balthasar 的模型名 |
| `AGENT2_MODEL_NAME` | CLI 中 Casper 的模型名 |
| `AGENT3_MODEL_NAME` | CLI 中 Melchior 的模型名 |
| `AGENT1_PREABLE` | 写作及评审基础提示词；工厂当前为所有身份使用它 |
| `AGENT2_PREABLE` | 预留的评审基础提示词，尚未被工厂使用 |
| `AGENT3_PREABLE` | 预留的核查基础提示词，尚未被工厂使用 |

`constance`、`PREABLE` 和 `agent::atuonomous` 都是现有命名，导入时保持原样。调整模型名或人格模板不会自动实现新业务模式。

## 接手时需先知道的边界

- 配置文件搜索、创建和加载仍是 `todo!()`；不要调用 `get_config_from_file`。
- `MagiCore::chat_completion` 和 `stream_completion` 尚未实现，调用会 panic。
- 人格前缀仍是占位文本，包括 Melchior 当前的 `harmful`；不能视为已完成 README 中的人格设计。
- 默认流程超时为 120 秒，可通过 `with_timeout` 调整。失败不返回部分结果，正常退出与失败清理详见 workflow 文档。
- `tests/agent_flow.rs` 覆盖流程和取消，`tests/bus.rs` 覆盖路由，`tests/mock_agent.rs` 覆盖执行循环，`tests/config.rs` 与配置模块单元测试覆盖模型映射。

后续修改公共接口时，同时更新该模块 README、Rustdoc 和对应测试。新增业务流程优先放在 `workflow`，不要把业务顺序重新放回 `main` 或 `bus`。
