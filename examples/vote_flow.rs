//! 运行：cargo run --example vote -- "是否发布新版本？"

use rust_magi::{
    agent::factory::{MagiCore, MagiCoreID},
    config::config::MagiConfiguration,
    constance::{
        AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME, CHEAP_MODEL_NAME,
        MAGI_GENERAL_PREABLE,
    },
    workflow::vote::VoteWorkflow,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    anyhow::ensure!(!query.trim().is_empty(), "请在命令行提供二元投票问题");
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
    let result = VoteWorkflow::new(
        balthasar.get_agent().clone(),
        casper.get_agent().clone(),
        melchior.get_agent().clone(),
    )
    .run(query)
    .await?;
    for vote in result.votes {
        println!("{}：{}，理由：{}", vote.id, vote.choice, vote.reason);
    }
    println!("多数决：{}", result.decision);
    Ok(())
}
