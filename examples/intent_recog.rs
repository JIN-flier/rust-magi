//! 临时文件，试一试意图识别的workflow

use rust_magi::{
    agent::{
        factory::MagiCoreID,
        intent::IntentWorkflow,
    },
    config::config::MagiConfiguration,
    constance::{AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME, CHEAP_MODEL_NAME},
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
        CHEAP_MODEL_NAME.to_string(),
    );
    let intent_agent: IntentWorkflow = IntentWorkflow::new(MagiCoreID::FastCheap, &config)?;

    let result = intent_agent.run("如何学习rust？").await?;
    println!("{}", result);

    let result = intent_agent.run("我连续健身了7周没有间断，今天十二有点儿多，时间安排不过来了，今天是否应该不去健身呢？").await?;
    println!("{}", result);

    let result = intent_agent.run("我连续健身了7周没有间断，今天十二有点儿多，时间安排不过来了，今天是否应该不去健身呢？告我是或否").await?;
    println!("{}", result);

    Ok(())
}
