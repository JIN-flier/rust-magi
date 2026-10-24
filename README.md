# MAGI

> 一个基于 [Rig](https://rig.rs/) 与 Rust 构建的多 Agent 协商和执行系统，灵感来自《新世纪福音战士》（EVA）的 MAGI 三贤人系统。

**项目状态：设计阶段（CLI 优先）**。本文记录当前的初步设计与预期行为，不代表下述功能已经实现。具体协议和细节会随着开发调整。

接手代码请从 [src 交接导航](src/README.md) 开始，其中链接了各模块的接口、调用示例、错误处理和未完成事项。

当前代码已实现一个文章评审示例：作者生成文章后，另外两个 Agent 并行给出改进建议和事实核查结果。该示例已拆分为以下模块：

- `bus::message::MessageBus`：管理独立的有界收件箱和公共响应入口；调用 `recv()` 时，将响应转发给其他 Agent，并交给工作流处理。
- `agent::atuonomous::AutonomousAgent`：执行 Task / Trigger，记录自身及其他 Agent 的响应，处理关闭和请求错误。
- `workflow::article_review::ArticleReviewWorkflow`：接收三个 Rig Agent，提供 `run(task)`，返回 `ArticleReviewResult { article, review, fact_check }`。
- `main`：加载配置、创建模型、调用工作流并打印三份结果。

工作流默认整体超时为 120 秒，可通过 `.with_timeout(Duration)` 调整。成功时发送 Shutdown 并等待退出；失败、超时或调用方取消时中止剩余任务。每次运行使用独立的总线和任务。Agent 的历史目前只在内存中记录，尚未传入模型上下文。

所有 Agent 身份统一使用 `agent::factory::MagiCoreID`：作者为 `Balthasar`，评审为 `Casper`，事实核查为 `Melchior`。总线注册、定向发送、收件箱及 `AgentMessage::Response` 都使用该枚举；日志通过其 `Display` 实现显示名称。

运行 `cargo test` 可执行不依赖 API Key 或外部服务的路由、Agent 和工作流测试；运行 `cargo run` 仍需现有 `.env` 和模型服务配置。以下 Vote / Discussion、工具调用及共识机制仍为规划内容。

## 1. 项目目标

MAGI 由三个能够独立思考、公开讨论和调用工具的子 Agent 组成。用户只需提交一个问题或任务，系统将：

1. 判断本次任务需要 **投票结论（`bool`）** 还是 **讨论结论（`String`）**。
2. 将相同的用户请求和必要上下文分别交给三个子 Agent。
3. 让三个子 Agent 通过公开消息频道交换观点，必要时自主调用工具获取信息或完成子任务。
4. 记录每个 Agent 的态度与完成状态，持续讨论直至满足结束条件。
5. 按任务模式产生最终结果：投票模式采用多数决；讨论模式由形成多数共识的 Agent 共同确定总结。

MAGI 的核心不只是并发调用三个大模型，而是让它们能够**表达不同意见、相互影响、使用工具，并共同形成可追踪的结果**。

## 2. 三贤人

| Agent | 人格定位 | 主要关注点 |
| --- | --- | --- |
| **MELCHIOR** | 科学家 | 事实、逻辑、技术可行性 |
| **BALTHASAR** | 母亲 | 安全、风险、长期影响 |
| **CASPER** | 个体 | 用户目标、实际需求、直觉与价值判断 |

三个 Agent 接收同一条原始 Query，但使用各自的人格提示词和独立上下文。人格只影响分析视角，不剥夺任何一个 Agent 的发言、投票或工具调用能力。

## 3. 任务模式

入口首先由 **Mode Classifier** 判定期望的最终输出类型：

| 模式 | 输出 | 适用问题 | 收敛规则 |
| --- | --- | --- | --- |
| **Vote** | `bool` | 是否接受、是否执行、是否满足某个条件 | 三个 Agent 分别提交 `true/false`，以多数票为准 |
| **Discussion** | `String` | 如何设计、如何分析、如何解决问题 | 讨论形成多数认可的方案，由多数同意者完成并确认总结 |

模式判断只决定**输出格式与收敛方式**，不影响 Agent 是否可以讨论或使用工具。即使是 Vote 模式，也可以先讨论、查证，再提交最终投票。

如果未能获得足够的有效投票或多数共识（例如 Agent 异常退出或达到轮数上限），任务返回“未完成/未达成共识”的状态及原因，不把失败伪装成 `false` 或已经达成一致的文本。

## 4. Agent 状态

每个子 Agent 维护两个**相互独立**的状态维度：

### 态度（Attitude）

- `Dissatisfied`（不满）：认为当前方案存在问题，或不接受其他 Agent 的意见。
- `Agree`（同意）：认可当前待决议方案或讨论结论。
- `Indifferent`（无所谓）：暂无明确立场、信息不足，或不反对但也未明确认可。

### 进度（Progress）

- `Unfinished`（还没完成）：仍在思考、发言、使用工具或等待其他 Agent。
- `Finished`（已经完成）：完成了当前任务的最终判断或总结确认。

两个状态可以独立变化。例如，`Dissatisfied + Unfinished` 表示正在提出异议；`Indifferent + Finished` 表示已经完成评估但仍不明确支持任何方案。

**态度不等于投票。** Vote 模式还需要每个 Agent 明确提交独立的 `bool` 票；Discussion 模式则需要确认 Agent 究竟同意的是哪一版候选结论，避免把对不同方案的“同意”错误地相加。

## 5. 通信：基于 Tokio mpsc 的公开讨论

每个 Agent 可以向**公共讨论频道**发送消息，其他 Agent 能看到公开发言，进而继续讨论。

由于 `tokio::sync::mpsc` 是多生产者、单消费者队列，本身并不是广播频道，初期采用**中心消息路由**：

```text
                  User Query
                      |
                Mode Classifier
                      |
                 Orchestrator
               /      |      \
         MELCHIOR BALTHASAR CASPER
               \      |      /
              mpsc 公共入口
                      |
                Message Router
               /      |      \
          Agent Inbox (各自 mpsc)
               \      |      /
                 三方可见
                      |
               Consensus Engine
                      |
                bool / String
```

通信约定：

1. 三个 Agent 均持有公共入口的 `mpsc::Sender<Event>`。
2. Orchestrator/Router 持有对应的 `Receiver<Event>`，负责接收、排序、记录公开事件。
3. Router 为每个 Agent 建立独立的收件队列，将公开消息**分别转发**给另外两个 Agent（必要时也同步发送者自身）。
4. Agent 收到新信息后可以更新态度、发言、调用工具或结束当前工作。
5. 消息带有 `task_id`、`agent_id`、`message_id`、时间戳等元数据，以支持后续调试与追踪。

公开频道发布的是**发言、结论、工具结果摘要和状态变化**，不要求暴露模型内部推理过程。为避免无限争论，Orchestrator 需要控制最大讨论轮数、消息数量以及超时。

## 6. 工具调用

三个 Agent **都拥有调用 Tools 的能力**，可以在讨论过程中主动查找证据或完成必要操作，例如：

- 文件读取、内容搜索、资料检索；
- 代码分析、运行测试、Git 操作；
- 后续按需接入的 MCP 工具及其他自定义工具。

预期流程：`Agent 请求 Tool → Tool Gateway 校验权限 → 执行 → 返回结果 → Agent 将相关发现公开到讨论频道`。

工具权限由运行时统一配置。初期优先实现安全的只读工具；对于写文件、执行命令、修改仓库或外部系统等操作，后续加入可配置审批机制。**能请求调用工具，不等于可以绕过授权直接执行任意操作。**

## 7. 协商与结束机制

### Vote 模式

1. 三个 Agent 收到相同问题并独立形成初步判断。
2. 可通过公共频道讨论、质疑和补充证据，也可调用工具。
3. 每个 Agent 提交最终 `true` 或 `false` 票，并标记 `Finished`。
4. 收到三个有效投票后，按 **2/3 多数决**输出最终 `bool`。

示例：`MELCHIOR=true`、`BALTHASAR=false`、`CASPER=true` → `true`。

### Discussion 模式

1. 三个 Agent 自由提出观点或解决方案，使用公开频道交流。
2. Agent 可对当前**同一版本**的候选结论表示 `Agree`、`Dissatisfied` 或 `Indifferent`。
3. 当至少两个 Agent 对同一候选结论表示 `Agree` 时，视为形成多数共识。
4. 在表示同意的 Agent 中指定一位总结者起草最终结论，其余同意者复核确认；需要修改则继续协商。
5. 结论确认后返回最终 `String`，并将本次参与 Agent 标记为 `Finished`。

未达成多数共识时不强行生成“共识结论”，而是保留分歧并交给上层继续处理。

> **说明：** `Finished` 表示 Agent 已完成当前职责，并不强制意味着它支持最终结论。保留少数派意见，避免把多数决误写成全员同意。

## 8. 运行流程

```text
用户输入
   ↓
模式分类（Vote / Discussion）
   ↓
并行启动三个 Rig Agent
   ↓
公开讨论（mpsc）←→ 工具调用
   ↓
更新态度与完成状态
   ↓
共识判断
   ├─ Vote：三个 bool 投票 → 多数决 → bool
   └─ Discussion：至少两个认可同一结论
                   ↓
              同意者总结并确认 → String
   ↓
CLI 输出结果与必要的过程日志
```

## 9. 计划中的项目结构

```text
magi/
├── Cargo.toml
├── README.md
├── config/
│   └── personas/             # 三贤人人格提示词
└── src/
    ├── main.rs               # CLI 入口
    ├── classifier.rs         # Vote / Discussion 判断
    ├── orchestrator.rs       # 生命周期与消息调度
    ├── agent.rs              # Rig Agent 封装、态度和进度
    ├── message.rs            # 消息类型、mpsc 路由
    ├── consensus.rs          # 多数投票和讨论收敛
    └── tools.rs              # 工具注册与调用入口
```

以上为建议的初期布局，后续可将模块拆分为目录或独立 crate。

## 10. CLI 交互草案

预期交互形式（**尚未实现的示例**）：

```bash
magi "这个方案是否值得执行？"
# Mode: Vote
# MELCHIOR: true
# BALTHASAR: false
# CASPER: true
# Result: true

magi "应该如何设计一个可扩展的代码研究 Agent？"
# Mode: Discussion
# [MELCHIOR] ...
# [BALTHASAR] ...
# [CASPER] ...
# Result: <得到多数认可的讨论总结>
```

CLI 初期将重点展示消息流、各 Agent 态度、完成状态、工具调用以及最终输出。未来可以增加交互式终端界面。

## 11. 开发计划

- [ ] 建立 Rust + Rig CLI 项目，加载三个独立 Agent。
- [ ] 实现用户请求的 Vote / Discussion 分类。
- [ ] 定义 Agent 的态度、进度以及公开消息协议。
- [ ] 实现 `mpsc` 公共入口和 Router 转发机制。
- [ ] 接入基础工具，让三个 Agent 都能主动调用。
- [ ] 实现 Vote 多数决、Discussion 多数共识与总结确认。
- [ ] 实现超时、轮数上限、异常退出与未达成共识处理。
- [ ] 加入日志与基础 CLI 交互。
- [ ] 后续扩展：权限审批、任务持久化、记忆、多种模型、更多工具及 UI。

## 12. 当前设计原则

- **三个独立人格**：共享任务，但不强制观点一致。
- **公开讨论**：通过公共消息通道交互，而不是一次并行问答。
- **工具平权**：每个 Agent 都可请求工具调用，实际权限由运行时统一控制。
- **状态显式化**：态度与完成进度分开管理。
- **结果可解释**：保留分歧、投票和关键发言。
- **CLI 优先**：先跑通最小闭环，再增加复杂特性。

---

**MAGI 的目标是让三个能够独立判断、交流和行动的 Agent，在受控的执行环境中形成可追踪的集体决策。**
