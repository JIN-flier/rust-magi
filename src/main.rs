//! 文章评审示例的命令行入口：加载环境、创建模型、调用库并打印结果。
//!
//! 当前写作任务固定在代码中，尚不解析命令行参数。

use rust_magi::{
    agent::factory::{MagiCore, MagiCoreID},
    config::config::MagiConfiguration,
    constance::{AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME},
    workflow::article_review::ArticleReviewWorkflow,
};

#[tokio::main]
/// 运行一次文章评审；`.env` 加载、模型初始化或工作流失败时向进程返回错误。
async fn main() -> anyhow::Result<()> {
    // 保持现有行为：找不到或无法解析 .env 也会使程序返回错误。
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
