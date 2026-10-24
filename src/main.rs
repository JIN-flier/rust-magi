use rust_magi::{
    agent::factory::{MagiCore, MagiCoreID},
    config::config::MagiConfiguration,
    constance::{AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME},
    workflow::article_review::ArticleReviewWorkflow,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let config = MagiConfiguration::new(
        AGENT1_MODEL_NAME.to_string(),
        AGENT2_MODEL_NAME.to_string(),
        AGENT3_MODEL_NAME.to_string(),
    );
    let writer = MagiCore::new(MagiCoreID::Balthasar, &config)?;
    let reviewer = MagiCore::new(MagiCoreID::Casper, &config)?;
    let fact_checker = MagiCore::new(MagiCoreID::Melchior, &config)?;

    let result = ArticleReviewWorkflow::new(
        writer.get_agent().clone(),
        reviewer.get_agent().clone(),
        fact_checker.get_agent().clone(),
    )
    .run("写一篇关于 Rust 所有权机制的简短技术介绍。")
    .await?;

    println!("Agent {} 完成文章:\n{}", writer.get_id(), result.article);
    println!("Agent {}:\n{}", reviewer.get_id(), result.review);
    println!("Agent {}:\n{}", fact_checker.get_id(), result.fact_check);
    Ok(())
}
