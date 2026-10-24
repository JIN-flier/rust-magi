//! 消息协议与中心路由：每个 Agent 拥有一个收件箱，响应经公共入口发布。
//!
//! `send` 是定向投递，`recv` 才驱动公开响应转发；本模块不会自动启动 Tokio 任务。

use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use tokio::sync::mpsc;

use crate::agent::factory::MagiCoreID;

/// Agent 与工作流之间交换的消息；身份统一使用 [`MagiCoreID`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMessage {
    /// 初始任务或用户提示词；接收方将其交给模型执行。
    Task(String),
    /// 公开响应，依次携带发送者身份和内容；接收方只记录它。
    Response(MagiCoreID, String),
    /// 后续任务提示词；当前执行方式与 Task 相同。
    Trigger(String),
    /// 请求接收方正常退出，不会抢占已经开始的模型请求。
    Shutdown,
    /// 通知接收方以给定原因失败退出；模型调用错误本身通过任务返回值上报。
    Error(String),
}

/// 与具体工作流无关的有界收件箱集合和中心响应路由。
///
/// 所有者必须调用 [`Self::recv`] 才会转发响应。慢消费者会使发送方等待队列空位，
/// 本模块不设置超时；流程结束条件、超时和任务清理由工作流负责。
pub struct MessageBus {
    senders: BTreeMap<MagiCoreID, mpsc::Sender<AgentMessage>>,
    mailboxes: BTreeMap<MagiCoreID, AgentMailbox>,
    events: mpsc::Receiver<AgentMessage>,
}

/// 一个 Agent 专属的收件箱及响应发布连接，不可克隆。
///
/// 丢弃此对象会关闭对应的接收端；丢弃总线后，已入队的消息仍可被读取完。
pub struct AgentMailbox {
    id: MagiCoreID,
    inbox: mpsc::Receiver<AgentMessage>,
    events: mpsc::Sender<AgentMessage>,
}

impl MessageBus {
    /// 为给定身份集合创建独立收件箱和一个公共事件队列。
    ///
    /// `capacity` 同时用于每个收件箱和公共队列；注册后不支持动态增删身份。
    /// 随后应通过 [`Self::take_mailbox`] 将每个收件箱交给对应的 Agent。
    ///
    /// # Errors
    ///
    /// 身份列表为空、身份重复或容量为 0 时返回错误。
    ///
    /// # Panics
    ///
    /// 容量超过 Tokio mpsc 支持的上限时，底层通道构造会 panic。
    pub fn new(agent_ids: &[MagiCoreID], capacity: usize) -> Result<Self> {
        ensure!(capacity > 0, "Message bus capacity must be positive");
        ensure!(
            !agent_ids.is_empty(),
            "Message bus needs at least one agent"
        );
        // 所有收件箱向同一个公共入口发布响应，由总线单点接收。
        let (events_tx, events) = mpsc::channel(capacity);
        let mut senders = BTreeMap::new();
        let mut mailboxes = BTreeMap::new();
        for &id in agent_ids {
            ensure!(!senders.contains_key(&id), "Duplicate agent ID: {id}");
            // 每个身份有独立的接收端；总线保留发送端用于定向投递。
            let (sender, inbox) = mpsc::channel(capacity);
            senders.insert(id, sender);
            mailboxes.insert(
                id,
                AgentMailbox {
                    id,
                    inbox,
                    events: events_tx.clone(),
                },
            );
        }
        Ok(Self {
            senders,
            mailboxes,
            events,
        })
    }

    /// 转移指定身份的收件箱所有权，每个收件箱只能领取一次。
    ///
    /// # Errors
    ///
    /// 身份未注册或收件箱已被领取时返回错误，不会自动创建新收件箱。
    pub fn take_mailbox(&mut self, id: MagiCoreID) -> Result<AgentMailbox> {
        self.mailboxes
            .remove(&id)
            .with_context(|| format!("Unknown or already taken agent mailbox: {id}"))
    }

    /// 只向指定身份投递消息；队列满时等待，不会向其他成员广播。
    ///
    /// # Errors
    ///
    /// 身份未注册或接收方已被丢弃时返回带身份信息的错误。
    pub async fn send(&self, id: MagiCoreID, message: AgentMessage) -> Result<()> {
        self.senders
            .get(&id)
            .with_context(|| format!("Unknown agent: {id}"))?
            .send(message)
            .await
            .with_context(|| format!("Agent {id} inbox closed"))
    }

    /// 接收一条公共响应，将其转发给除发送者外的所有成员，再返回给调用方。
    ///
    /// 转发按路由表顺序逐个等待。此方法不保证取消安全：事件出队后取消，可能
    /// 已完成部分转发，而调用方尚未收到返回值；不要将其当成可随时重试的轮询。
    ///
    /// # Errors
    ///
    /// 公共队列已关闭且排空，或任一转发目标关闭时返回错误。转发失败前可能已有
    /// 成员收到消息；公共队列仅在全部收件箱（含未领取的收件箱）丢弃后关闭。
    pub async fn recv(&mut self) -> Result<AgentMessage> {
        let message = self
            .events
            .recv()
            .await
            .context("Bus event channel closed")?;
        // 发送者已经保存自身输出，不回送，以免重复记录。
        if let AgentMessage::Response(from, _) = &message {
            for id in self.senders.keys().filter(|id| *id != from) {
                self.send(*id, message.clone()).await?;
            }
        }
        Ok(message)
    }

    /// 向全部收件箱发送 Shutdown；遇到关闭的收件箱后仍尝试其余成员。
    ///
    /// 只负责消息入队，不关闭通道、不等待 Agent 退出，也不强制取消任务。
    /// 队列满时仍会等待，调用方应提供超时及任务回收机制。
    ///
    /// # Errors
    ///
    /// 尝试完所有成员后返回遇到的第一个发送错误；全部发送成功则返回 `Ok(())`。
    pub async fn shutdown(&self) -> Result<()> {
        let mut first_error = None;
        for id in self.senders.keys() {
            if let Err(error) = self.send(*id, AgentMessage::Shutdown).await {
                first_error.get_or_insert(error);
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl AgentMailbox {
    /// 返回收件箱在注册时绑定的身份。
    pub fn id(&self) -> MagiCoreID {
        self.id
    }

    /// 等待下一条定向消息或转发响应；所有发送端关闭且缓冲区排空后返回 `None`。
    pub async fn recv(&mut self) -> Option<AgentMessage> {
        self.inbox.recv().await
    }

    /// 将内容和自身绑定的身份发布到公共队列，发送者不能通过此方法指定其他身份。
    ///
    /// 成功仅表示已入队，仍需所有者调用 [`MessageBus::recv`] 才会转发给其他成员。
    /// 公共队列满时等待空位。
    ///
    /// # Errors
    ///
    /// 总线的公共接收端被丢弃时返回错误。
    pub async fn respond(&self, content: String) -> Result<()> {
        self.events
            .send(AgentMessage::Response(self.id, content))
            .await
            .context("Bus event channel closed")
    }
}
