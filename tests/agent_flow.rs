
// use std::time::Duration;

// use rig::prelude::*;
// use rig::test_utils::MockCompletionModel;

// #[tokio::test]
// async fn mock_three_agent_workflow_completes() {
//     const ARTICLE: &str =
//         "MOCK_ARTICLE: Rust ownership test article";

//     const REVIEW: &str =
//         "MOCK_REVIEW: three concrete suggestions";

//     const FACT_CHECK: &str =
//         "MOCK_FACT_CHECK: factual verification result";

//     // 创建三个独立的 Mock 模型。
//     let model1 = MockCompletionModel::text(ARTICLE);
//     let model2 = MockCompletionModel::text(REVIEW);
//     let model3 = MockCompletionModel::text(FACT_CHECK);

//     // 保留观察句柄，用于检查实际发送给模型的请求。
//     MockCompletionModel::clone(&model1);
//     let probe1 = model1.clone();
//     let probe2 = model2.clone();
//     let probe3 = model3.clone();

//     let agent1: Agent = AgentBuilder::new(model1)
//         .preamble("Mock Agent 1")
//         .build();

//     let agent2: Agent = AgentBuilder::new(model2)
//         .preamble("Mock Agent 2")
//         .build();

//     let agent3: Agent = AgentBuilder::new(model3)
//         .preamble("Mock Agent 3")
//         .build();

//     // 整个流程不访问任何真实 Provider。
//     let result = tokio::time::timeout(
//         Duration::from_secs(10),
//         run_workflow(agent1, agent2, agent3),
//     )
//     .await
//     .expect("工作流不应该无限等待")
//     .expect("三个 Agent 应该正常完成工作流");

//     // 检查三份输出。
//     assert_eq!(result.article, ARTICLE);
//     assert_eq!(result.review, REVIEW);
//     assert_eq!(result.fact_check, FACT_CHECK);

//     // 检查每个模型都被调用了一次。
//     assert_eq!(probe1.request_count(), 1);
//     assert_eq!(probe2.request_count(), 1);
//     assert_eq!(probe3.request_count(), 1);

//     // 检查 Agent 2 确实收到了文章。
//     let request2 = format!("{:?}", probe2.requests());
//     assert!(
//         request2.contains(ARTICLE),
//         "Agent 2 的请求中没有包含 Agent 1 的文章"
//     );

//     // 检查 Agent 3 确实收到了文章。
//     let request3 = format!("{:?}", probe3.requests());
//     assert!(
//         request3.contains(ARTICLE),
//         "Agent 3 的请求中没有包含 Agent 1 的文章"
//     );

//     println!("Mock 多 Agent 工作流测试通过");
// }
