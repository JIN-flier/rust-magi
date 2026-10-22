use rig::prelude::*;
use rig::providers::openai::OpenAI;
use rust_magi::constance::{
    AGENT1_MODEL_NAME, AGENT1_PREABLE, AGENT2_MODEL_NAME, AGENT2_PREABLE, AGENT3_MODEL_NAME,
    AGENT3_PREABLE,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, interval};

/// Messages exchanged between agents.
#[derive(Debug, Clone)]
enum AgentMessage {
    Task(String),
    Response(String, String), // (from_agent_id, content)
    Trigger(String),
    Shutdown,
}

/// An actor-based autonomous agent. Each holds its own Rig agent, an inbox,
/// its history, and channels to its peers.
struct AutonomousAgent {
    id: String,
    agent: Agent,
    history: Vec<String>,
    inbox: mpsc::Receiver<AgentMessage>,
    peers: Arc<RwLock<Vec<mpsc::Sender<AgentMessage>>>>,
}

impl AutonomousAgent {
    /// Broadcast a message to every registered peer.
    async fn broadcast(&self, message: AgentMessage) {
        for peer in self.peers.read().await.iter() {
            let _ = peer.send(message.clone()).await;
        }
    }

    async fn run(mut self) {
        let mut tick = interval(Duration::from_secs(10));
        loop {
            tokio::select! {
                Some(msg) = self.inbox.recv() => match msg {
                    AgentMessage::Shutdown => {
                        eprintln!("[Agent {}] 收到 Shutdown", self.id);
                        break;
                    }
                    AgentMessage::Task(task) => {
                        if let Ok(response) = self.agent.prompt(task.clone()).await {
                            let result = response.output().to_string();
                            self.history.push(result.clone());
                            self.broadcast(AgentMessage::Response(self.id.clone(), result.clone())).await;
                            println!("ID: {} \n task: {} \n result: {} \n", self.id, task, result);
                        } else {
                            eprintln!("error ocur in Task");
                        }
                    }
                    AgentMessage::Trigger(msg) => {
                        if let Ok(response) = self.agent.prompt(msg.clone()).await {
                            let result = response.output().to_string();
                            self.history.push(result.clone());
                            self.broadcast(AgentMessage::Response(self.id.clone(),result.clone())).await;
                            println!("ID: {} \n msg: {} \n result: {} \n", self.id, msg, result);
                        } else {
                            eprintln!("error ocur in Trigger");
                        }
                    }
                    AgentMessage::Response(from, content) => {
                        self.history.push(format!("From {from}: {content}"));
                    }
                },
                // Autonomous periodic self-check (external trigger)
                _ = tick.tick() => {
                    // e.g. summarize progress, enqueue follow-up work, etc.
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let agent1: Agent = AgentBuilder::new(OpenAI::from_env()?.completion(AGENT1_MODEL_NAME))
        .preamble(AGENT1_PREABLE)
        .build();
    let agent2: Agent = AgentBuilder::new(OpenAI::from_env()?.completion(AGENT2_MODEL_NAME))
        .preamble(AGENT2_PREABLE)
        .build();
    let agent3: Agent = AgentBuilder::new(OpenAI::from_env()?.completion(AGENT3_MODEL_NAME))
        .preamble(AGENT3_PREABLE)
        .build();

    // Create independent inboxes.
    let (tx_1, rx_1) = mpsc::channel(64);
    let (tx_2, rx_2) = mpsc::channel(64);
    let (tx_3, rx_3) = mpsc::channel(64);
    let (tx_main, mut rx_main) = mpsc::channel(64);

    // Configure peer connections.
    let agent1_peers = Arc::new(RwLock::new(vec![
        tx_2.clone(),
        tx_3.clone(),
        tx_main.clone(),
    ]));

    let agent2_peers = Arc::new(RwLock::new(vec![
        tx_1.clone(),
        tx_3.clone(),
        tx_main.clone(),
    ]));

    let agent3_peers = Arc::new(RwLock::new(vec![
        tx_2.clone(),
        tx_1.clone(),
        tx_main.clone(),
    ]));

    let agent1: AutonomousAgent = AutonomousAgent {
        id: "1".to_string(),
        agent: agent1,
        history: Vec::new(),
        inbox: rx_1,
        peers: agent1_peers,
    };
    let agent2: AutonomousAgent = AutonomousAgent {
        id: "2".to_string(),
        agent: agent2,
        history: Vec::new(),
        inbox: rx_2,
        peers: agent2_peers,
    };
    let agent3: AutonomousAgent = AutonomousAgent {
        id: "3".to_string(),
        agent: agent3,
        history: Vec::new(),
        inbox: rx_3,
        peers: agent3_peers,
    };

    let agent1_handler = tokio::spawn(agent1.run());
    let agent2_handler = tokio::spawn(agent2.run());
    let agent3_handler = tokio::spawn(agent3.run());

    // 1. 让 Agent 1 生成文章
    tx_1.send(AgentMessage::Task(
        "写一篇关于 Rust 所有权机制的简短技术介绍。".to_string(),
    ))
    .await?;

    // 2. 等待 Agent 1 返回文章
    let article = loop {
        match rx_main.recv().await {
            Some(AgentMessage::Response(from, content)) if from == "1" => {
                break content;
            }
            Some(_) => {}
            None => anyhow::bail!("Main message channel closed"),
        }
    };

    println!("Agent 1 完成文章:\n{article}");

    println!("正在向 Agent 2 发送评审任务");
    // 3. 将文章交给 Agent 2 和 Agent 3 并行处理
    tx_2.send(AgentMessage::Trigger(format!(
        "请评审下面的技术文章，提出三条具体改进建议。\n\n\
     文章内容：\n{article}"
    )))
    .await?;

    println!("正在向 Agent 3 发送评审任务");
    tx_3.send(AgentMessage::Trigger(format!(
        "请检查下面的技术文章是否存在知识错误，\
     如果有则提出具体建议。\n\n\
     文章内容：\n{article}"
    )))
    .await?;

    let mut finished = HashSet::new();
    while finished.len() < 2 {
        match rx_main.recv().await {
            Some(AgentMessage::Response(from, content)) if from == "2" || from == "3" => {
                println!("Agent {from}:\n{content}");
                finished.insert(from);
            }
            Some(_) => {}
            None => anyhow::bail!("Main message channel closed"),
        }
    }

    tx_1.send(AgentMessage::Shutdown).await?;
    tx_2.send(AgentMessage::Shutdown).await?;
    tx_3.send(AgentMessage::Shutdown).await?;

    agent1_handler.await?;
    agent2_handler.await?;
    agent3_handler.await?;

    Ok(())
}
