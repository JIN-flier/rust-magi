# Workflow / bus 解耦实施计划

> 使用 writing-plans、executing-plans 和 test-driven-development 技能在当前会话执行；按用户要求，修改后交用户审阅，保留未提交 diff。

**目标：** 将已有文章生成和双路评审流程从 main 移入可测试的库模块。

**设计：** bus 提供有界 mpsc 收件箱、定向发送和公开响应转发；独立的 AutonomousAgent 消费收件箱并执行 Rig 请求；ArticleReviewWorkflow 负责三名 Agent 的生命周期、请求顺序和结果汇总。main 只创建配置与模型、调用流程、输出结果。

**技术：** 使用现有 Rust / Tokio / Rig / anyhow，不新增依赖。

**方案取舍：** 仅搬移 main 函数不能独立测试路由；直接并发调用三个模型会丢失现有 Agent 间的消息交换。因此采用模块封装和中心转发，在 workflow 接收事件时将 Response 分发给其他 Agent。沿用原有 AgentMessage 协议和三个角色；暂不实现 README 中规划的投票和共识功能。

**约束：** 保持写作与评审提示词及模型配置；沿用现有 atuonomous 模块名称，避免无关 API 改名；不提交、不推送，等待用户代码审阅。用户要求修改后审阅，因此不增加实施前审批步骤。

## 审阅重点

- 响应只转发给其他 Agent，定向任务不广播。
- 非法 ID、重复领取、断开的收件箱及事件通道返回明确错误。
- 两名评审都收到作者输出，输出按角色归位，且同时执行。
- 任一模型失败、Agent panic、流程超时均结束任务并回收后台任务。
- 调用方取消流程时，后台任务不能脱离工作流继续运行。

## 任务 1：消息总线

- [x] 在 tests/bus.rs 中覆盖定向消息、广播排除自身、未知/重复 ID、关闭通道。
- [x] 运行 cargo test --test bus，确认新接口缺失。
- [x] 实现 MessageBus::new(&[MagiCoreID], usize)、take_mailbox、send、recv、shutdown；AgentMailbox 提供 id、recv、respond。
- [x] 运行 bus 测试，确认通过。

## 任务 2：Agent 和 workflow

- [x] 将 tests/agent_flow.rs 中注释测试替换为真实工作流测试，补失败和生命周期测试。
- [x] 运行测试，确认工作流接口缺失。
- [x] 将执行循环移至 src/agent/atuonomous.rs；请求失败返回带 Agent ID 的错误，收件箱关闭或 Shutdown 正常退出。
- [x] 在 src/workflow/article_review.rs 实现 ArticleReviewWorkflow::new(Agent, Agent, Agent)、with_timeout(Duration)、run(task)，返回 ArticleReviewResult { article, review, fact_check }。
- [x] 使用 JoinSet 管理任务；默认 120 秒超时可配置，成功发送 Shutdown 并等待，失败中止并等待，外部取消依靠 JoinSet 的 drop 中止。
- [x] 运行工作流与 Agent 测试，确认通过。

## 任务 3：入口与验证

- [x] 简化 src/main.rs，增加库调用说明。
- [x] 运行 cargo test、cargo clippy --all-targets、格式检查和 git diff --check。
- [x] 独立审阅最终 diff，记录已存在的警告及验证结果；将修改交用户审阅。

## 验证结果

- 基线：2 个测试通过；原有 workflow 测试均为注释。
- 新接口测试先因接口尚未实现而失败，完成封装后通过。
- 最终 `cargo test`：19 个测试通过（17 个新增），无外部模型调用。
- `cargo clippy --all-targets`：退出码 0；仅保留未修改文件中的 11 条既有警告。
- 对所有修改的 Rust 文件运行 `rustfmt --check --edition 2024`，通过；`git diff --check` 通过。
- 独立代码审阅：未发现需要修复的正确性问题。
- 尚未单独强制覆盖的场景：评审的两种完成顺序、同一 workflow 的多次/并发调用、队列满时的背压、部分 Agent 断开后的其余 Shutdown 投递。实际模型端到端调用未执行。
- 修改保留在工作区，未提交或推送，等待用户审阅。

## 审阅调整：统一身份枚举

- 按用户要求，将所有 Agent ID 从字符串替换为 `MagiCoreID`，包括消息发送者、路由键、收件箱、公共方法参数及工作流角色。
- `MagiCoreID` 增加 Copy、Ord 及 Display 支持，CLI 和错误消息也使用枚举名称。
- 沿用用户调整后的 `bus/message.rs` 文件布局；空字符串 ID 已由类型系统排除，继续测试未注册成员和重复 ID。
- 测试先在字符串接口上因类型不匹配失败；修改后 `cargo test` 全部 19 个测试通过。Clippy 无新增警告，仅保留 10 条既有警告。
