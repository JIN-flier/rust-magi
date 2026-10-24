//! 用三个独立的消息驱动 Agent 完成“写文章 → 并行评审和事实核查”。
//!
//! 业务顺序由工作流管理，通信由总线管理，实际模型请求由 Agent 管理。

use std::time::Duration;

use anyhow::{Context, Result, bail};
use rig::prelude::*;
use tokio::{task::JoinSet, time::timeout};

use crate::{
    agent::{atuonomous::AutonomousAgent, factory::MagiCoreID},
    bus::message::{AgentMessage, MessageBus},
};

/// 本流程的作者身份；不依赖枚举声明顺序。
const WRITER: MagiCoreID = MagiCoreID::Balthasar;
/// 本流程的改进建议评审者身份。
const REVIEWER: MagiCoreID = MagiCoreID::Casper;
/// 本流程的事实核查者身份。
const FACT_CHECKER: MagiCoreID = MagiCoreID::Melchior;

/// 三个步骤都成功后返回的文本结果；不表示已实现投票或共识判断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleReviewResult {
    /// Balthasar 生成的文章原文。
    pub article: String,
    /// Casper 给出的改进建议。
    pub review: String,
    /// Melchior 给出的事实核查结果。
    pub fact_check: String,
}

/// 先生成文章，再并行执行评审和事实核查。
///
/// 注入 Rig Agent 以支持实际服务端或离线 Mock。每次运行创建独立的总线和后台任务；
/// 复用此对象时，底层模型句柄仍由 Rig 的克隆语义决定，不保证模型内部状态隔离。
pub struct ArticleReviewWorkflow {
    writer: Agent,
    reviewer: Agent,
    fact_checker: Agent,
    timeout: Duration,
}

impl ArticleReviewWorkflow {
    /// 按作者、改进建议评审者、事实核查者的顺序保存三个 Rig Agent。
    ///
    /// 默认超时为 120 秒。构造时不创建任务、不加载环境，也不调用模型。
    /// 工作流按参数位置绑定身份，不检查传入 Agent 的系统提示词是否匹配该角色。
    pub fn new(writer: Agent, reviewer: Agent, fact_checker: Agent) -> Self {
        Self {
            writer,
            reviewer,
            fact_checker,
            timeout: Duration::from_secs(120),
        }
    }

    /// 设置业务执行、消息投递和正常退出的超时时长，并返回更新后的工作流。
    ///
    /// 超时后仍会取消并等待剩余任务退出；该清理过程不在超时窗口内。
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 以 `task` 作为作者提示词，运行一次完整流程并返回三份文本结果。
    ///
    /// 使用容量 64 的消息队列，成功时发送 Shutdown 并等待所有 Agent 退出。
    /// 失败或超时会中止并等待剩余任务；调用方丢弃此 future 时，JoinSet 会请求
    /// 中止所拥有的任务。任务取消遵循 Tokio 协作式调度，无法抢占同步阻塞代码。
    ///
    /// # Errors
    ///
    /// 模型请求或路由失败、Agent 提前退出/panic、流程超时时返回错误。
    /// 失败不返回部分结果；方法自身不打印业务输出。
    pub async fn run(&self, task: impl Into<String>) -> Result<ArticleReviewResult> {
        let mut bus = MessageBus::new(&[WRITER, REVIEWER, FACT_CHECKER], 64)?;
        // 将任务所有权绑定到本次调用，避免调用方取消后遗留脱离管理的任务。
        let mut agents = JoinSet::new();
        for (id, agent) in [
            (WRITER, &self.writer),
            (REVIEWER, &self.reviewer),
            (FACT_CHECKER, &self.fact_checker),
        ] {
            let actor = AutonomousAgent::new(agent.clone(), bus.take_mailbox(id)?);
            agents.spawn(actor.run());
        }

        let mut result = timeout(self.timeout, async {
            let result = tokio::select! {
                biased;
                completed = agents.join_next() => {
                    completed.context("No agent tasks running")?
                        .context("Agent task failed")??;
                    bail!("Agent exited before workflow completed");
                }
                result = Self::exchange(&mut bus, task.into()) => result?,
            };
            bus.shutdown().await?;
            while let Some(completed) = agents.join_next().await {
                completed.context("Agent task failed")??;
            }
            Ok(result)
        })
        .await
        .context("Article review workflow timed out")
        .and_then(|result| result);

        // 失败时中止并回收剩余任务。若路由先观察到收件箱关闭，保留随后取到的
        // 原始模型错误或 panic，避免只向调用方报告一个次生的通道错误。
        agents.abort_all();
        while let Some(completed) = agents.join_next().await {
            match completed {
                Ok(Err(error)) => result = Err(error),
                Err(error) if !error.is_cancelled() => {
                    result = Err(error).context("Agent task failed");
                }
                _ => {}
            }
        }
        result
    }

    /// 编排请求并按身份收集结果；后台任务的监控和清理由 `run` 负责。
    async fn exchange(bus: &mut MessageBus, task: String) -> Result<ArticleReviewResult> {
        bus.send(WRITER, AgentMessage::Task(task)).await?;
        let article = loop {
            match bus.recv().await? {
                AgentMessage::Response(from, content) if from == WRITER => break content,
                AgentMessage::Error(error) => bail!("{error}"),
                _ => {}
            }
        };

        // 这里只等待任务入队；两个 Agent 将并行执行，不等待第一份评审完成。
        bus.send(
            REVIEWER,
            AgentMessage::Trigger(format!(
                "请评审下面的技术文章，提出三条具体改进建议。\n\n文章内容：\n{article}"
            )),
        )
        .await?;
        bus.send(FACT_CHECKER, AgentMessage::Trigger(format!(
            "请检查下面的技术文章是否存在知识错误，如果有则提出具体建议。\n\n文章内容：\n{article}"
        ))).await?;

        // 按身份而非到达顺序归位，避免完成较快的核查结果被当作评审结果。
        let mut review = None;
        let mut fact_check = None;
        while review.is_none() || fact_check.is_none() {
            match bus.recv().await? {
                AgentMessage::Response(from, content) if from == REVIEWER => review = Some(content),
                AgentMessage::Response(from, content) if from == FACT_CHECKER => {
                    fact_check = Some(content)
                }
                AgentMessage::Error(error) => bail!("{error}"),
                _ => {}
            }
        }
        Ok(ArticleReviewResult {
            article,
            review: review.context("Reviewer produced no result")?,
            fact_check: fact_check.context("Fact checker produced no result")?,
        })
    }
}
