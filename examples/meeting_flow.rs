//! 运行：cargo run --example meeting -- "如何发布新版本？"

use rust_magi::{
    agent::factory::{MagiCore, MagiCoreID},
    config::config::MagiConfiguration,
    constance::{
        AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME, CHEAP_MODEL_NAME,
        MAGI_GENERAL_PREABLE,
    },
    workflow::meeting::MeetingWorkflow,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    anyhow::ensure!(!query.trim().is_empty(), "请在命令行提供会议问题");
    let config = MagiConfiguration::new(
        AGENT1_MODEL_NAME.into(),
        AGENT2_MODEL_NAME.into(),
        AGENT3_MODEL_NAME.into(),
        CHEAP_MODEL_NAME.into(),
    );
    let balthasar =
        MagiCore::new_with_preamble(MagiCoreID::Balthasar, &config, MAGI_GENERAL_PREABLE)?;
    let casper = MagiCore::new_with_preamble(MagiCoreID::Casper, &config, MAGI_GENERAL_PREABLE)?;
    let melchior =
        MagiCore::new_with_preamble(MagiCoreID::Melchior, &config, MAGI_GENERAL_PREABLE)?;
    let result = MeetingWorkflow::new(
        balthasar.get_agent().clone(),
        casper.get_agent().clone(),
        melchior.get_agent().clone(),
    )
    .run(query)
    .await?;
    for (index, round) in result.rounds.iter().enumerate() {
        println!("第 {} 轮，候选结论：{:?}", index + 1, round.candidate);
        for message in &round.messages {
            println!(
                "{}：{}；提案：{}（认可候选：{}）",
                message.id, message.message, message.proposal, message.agrees_with_candidate
            );
        }
    }
    match result.consensus {
        Some(consensus) => println!("共同意见：{consensus}"),
        None => println!("达到最大轮数，尚未取得三人共同意见"),
    }
    Ok(())
}
