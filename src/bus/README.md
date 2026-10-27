# bus：消息协议与中心响应路由

[返回 src 导航](../README.md)

实现集中在 [message.rs](message.rs)，导入路径为 `rust_magi::bus::message::{AgentMessage, MessageBus, AgentMailbox}`。总线不调用模型、不启动后台任务，也不决定工作流何时完成。

## 消息接口：AgentMessage

| 变体 | 含义 | 当前 AutonomousAgent 的行为 |
| --- | --- | --- |
| `Task(String)` | 初始任务提示词 | 请求模型 |
| `Trigger(String)` | 后续任务提示词 | 请求模型，与 Task 执行方式相同 |
| `Response(MagiCoreID, String)` | 发送者身份和公开文本 | 记录历史，不触发模型 |
| `Shutdown` | 正常退出请求 | 当前消息处理结束后退出 |
| `Error(String)` | 带原因的失败通知 | 以错误退出 |

身份只能使用 `MagiCoreID`。`Error` 不带独立发送者字段；模型错误由 actor 的返回值上报，工作流通过 JoinSet 观察。

## 总线接口：MessageBus

下表省略的 `Result` 错误类型均为 `anyhow::Error`；标注 async 的方法需要 `.await`。

| 接口 | 功能 | 错误条件 |
| --- | --- | --- |
| `new(agent_ids: &[MagiCoreID], capacity: usize) -> Result<Self>` | 创建每个身份的独立收件箱及公共事件队列，二者均采用 capacity | 空身份列表、重复身份、容量为 0 |
| `take_mailbox(&mut self, id: MagiCoreID) -> Result<AgentMailbox>` | 将收件箱所有权交给调用方，只能领取一次 | 身份未注册或已领取 |
| `send(&self, id: MagiCoreID, message: AgentMessage) -> Result<()>`（async） | 只投递到指定收件箱 | 身份未注册或接收端关闭 |
| `recv(&mut self) -> Result<AgentMessage>`（async） | 接收公共响应，转发给其他注册成员后返回响应 | 公共入口关闭且排空，或转发到关闭的收件箱 |
| `shutdown(&self) -> Result<()>`（async） | 尝试向所有成员发送 Shutdown | 尝试完后返回首个发送错误 |

注册集合创建后固定，不支持动态增加/删除成员。容量超过 Tokio 通道支持的上限会由底层构造触发 panic。

## 收件箱接口：AgentMailbox

| 接口 | 功能 |
| --- | --- |
| `id(&self) -> MagiCoreID` | 返回注册时绑定的身份 |
| `recv(&mut self) -> Option<AgentMessage>`（async） | 接收定向任务或转发响应；发送端全部关闭且缓冲区排空后返回 None |
| `respond(&self, content: String) -> Result<()>`（async） | 自动附上绑定身份并发布到公共队列；总线已被丢弃则报错 |

`respond` 不能指定其他发送者身份。入队成功不表示其他成员已经收到消息。

## 路由与生命周期约定

```text
工作流 -- send(id, message) --> 指定 AgentMailbox::recv
Agent -- mailbox.respond(content) --> 公共事件队列
工作流 -- bus.recv() --> 向其他 Agent 的收件箱转发 --> 返回响应给工作流
```

- 所有者必须持续驱动 `bus.recv()`，总线没有自动运行的路由线程。
- 只有经 `respond` 发布到公共入口的响应才会被 `bus.recv()` 转发；直接 `send` 一个 Response 也仍然是定向投递。
- 发送者不接收自己的广播。转发逐个等待，遇到关闭的接收端即返回错误，之前的成员可能已收到消息。
- 所有队列都有界，队列满会等待。总线不内置超时，业务所有者应同时驱动消费者并提供超时。
- `bus.recv()` 不是取消安全的轮询：在出队和逐个转发之间取消，可能导致部分投递而调用方未收到返回值。当前投票和会议工作流不使用总线；直接使用总线的调用方需自行处理取消。
- `shutdown()` 仅入队，不关闭通道、不等待退出、不抢占模型请求。遇到已关闭成员仍会尝试其他成员，但满队列仍可能阻塞后续发送。
- 丢弃总线会关闭收件箱的发送端；已入队消息仍可排空。公共入口只有在所有 AgentMailbox（包括总线中尚未领取的）被丢弃后才关闭。

## 最小路由示例

```rust
use rust_magi::{
    agent::factory::MagiCoreID::{Balthasar, Casper},
    bus::message::{AgentMessage, MessageBus},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut bus = MessageBus::new(&[Balthasar, Casper], 8)?;
    let writer = bus.take_mailbox(Balthasar)?;
    let mut reviewer = bus.take_mailbox(Casper)?;
    writer.respond("文章".into()).await?;
    let response = bus.recv().await?; // 驱动转发
    assert_eq!(response, AgentMessage::Response(Balthasar, "文章".into()));
    assert_eq!(reviewer.recv().await, Some(response));
    Ok(())
}
```

运行 `cargo test --test bus` 验证定向投递、排除自身、非法拓扑、重复领取和通道关闭。增加新的消息种类时，同步检查 actor 的匹配分支和 workflow 的结束条件。
