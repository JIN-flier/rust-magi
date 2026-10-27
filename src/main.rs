//! 文章评审示例的命令行入口：加载环境、创建模型、调用库并打印结果。
//!
//! 当前写作任务固定在代码中，尚不解析命令行参数。

use rust_magi::{
    agent::{
        factory::{MagiCore, MagiCoreID},
        intent::{IntentWorkflow, Intention},
    }, config::config::MagiConfiguration, constance::{
        AGENT1_MODEL_NAME, AGENT2_MODEL_NAME, AGENT3_MODEL_NAME, CHEAP_MODEL_NAME,
        MAGI_GENERAL_PREABLE,
    }, workflow::{meeting::MeetingWorkflow, vote::VoteWorkflow},
};
use rust_magi::workflow::result::WorkflowResult;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let config = MagiConfiguration::new(
        AGENT1_MODEL_NAME.to_string(),
        AGENT2_MODEL_NAME.to_string(),
        AGENT3_MODEL_NAME.to_string(),
        CHEAP_MODEL_NAME.to_string(),
    );

    let balthasar =
        MagiCore::new_with_preamble(MagiCoreID::Balthasar, &config, MAGI_GENERAL_PREABLE)?;
    let casper = MagiCore::new_with_preamble(MagiCoreID::Casper, &config, MAGI_GENERAL_PREABLE)?;
    let melchior =
        MagiCore::new_with_preamble(MagiCoreID::Melchior, &config, MAGI_GENERAL_PREABLE)?;

    let intent_workflow: IntentWorkflow = IntentWorkflow::new(MagiCoreID::FastCheap, &config)?;

    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    // let query = "如何学习rust？";
    // let query = "我连续健身了7周没有间断，今天十二有点儿多，时间安排不过来了，今天是否应该不去健身呢？";
    // let query = "我连续健身了7周没有间断，今天十二有点儿多，时间安排不过来了，今天是否应该不去健身呢？告我是或否";
    anyhow::ensure!(!query.trim().is_empty(), "请在命令行提供会议问题");

    let intent = intent_workflow.run(&query).await?;
    println!("{}", intent);

    let workflow_result: Box<dyn WorkflowResult> = match intent {
        Intention::Vote { option_a, option_b } => {
            let result = VoteWorkflow::new(
                balthasar.get_agent().clone(),
                casper.get_agent().clone(),
                melchior.get_agent().clone(),
            )
            .run(query, &Intention::Vote{option_a, option_b})
            .await?;
            Box::new(result)
        }
        Intention::Meeting { interpretation } => {
            let result = MeetingWorkflow::new(
                balthasar.get_agent().clone(),
                casper.get_agent().clone(),
                melchior.get_agent().clone(),
            )
            .run(query)
            .await?;
            Box::new(result)
        }
    };
    println!("{}", workflow_result.show());

    Ok(())
}
