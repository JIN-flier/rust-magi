use std::time::Duration;

use rig::{
    prelude::*,
    test_utils::{MockCompletionModel, MockTurn},
};
use rust_magi::{agent::factory::MagiCoreID, workflow::vote::VoteWorkflow};

fn agent(model: MockCompletionModel) -> Agent {
    AgentBuilder::new(model).build()
}

#[tokio::test]
async fn collects_three_schema_votes_and_returns_majority() {
    let balthasar = MockCompletionModel::text(r#"{"choice":true,"reason":"safe"}"#);
    let casper = MockCompletionModel::text(r#"{"choice":false,"reason":"too costly"}"#);
    let melchior = MockCompletionModel::text(r#"{"choice":true,"reason":"feasible"}"#);
    let result = VoteWorkflow::new(
        agent(balthasar.clone()),
        agent(casper.clone()),
        agent(melchior.clone()),
    )
    .with_timeout(Duration::from_secs(3))
    .run("Should we proceed?")
    .await
    .unwrap();

    assert!(result.decision);
    assert_eq!(result.votes.len(), 3);
    assert_eq!(result.votes[0].id, MagiCoreID::Balthasar);
    assert_eq!(result.votes[1].id, MagiCoreID::Casper);
    assert_eq!(result.votes[2].id, MagiCoreID::Melchior);
    assert_eq!(result.votes[1].reason, "too costly");
    for model in [&balthasar, &casper, &melchior] {
        assert_eq!(model.request_count(), 1);
        assert!(format!("{:?}", model.requests()).contains("Should we proceed?"));
    }
}

#[tokio::test]
async fn false_majority_is_false() {
    let result = VoteWorkflow::new(
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"a"}"#,
        )),
        agent(MockCompletionModel::text(r#"{"choice":true,"reason":"b"}"#)),
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"c"}"#,
        )),
    )
    .run("question")
    .await
    .unwrap();
    assert!(!result.decision);
}

#[tokio::test]
async fn invalid_vote_or_model_failure_does_not_produce_a_decision() {
    let invalid = VoteWorkflow::new(
        agent(MockCompletionModel::text(r#"{"choice":true,"reason":"a"}"#)),
        agent(MockCompletionModel::text("yes")),
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"c"}"#,
        )),
    );
    assert!(invalid.run("question").await.is_err());

    let failed = VoteWorkflow::new(
        agent(MockCompletionModel::text(r#"{"choice":true,"reason":"a"}"#)),
        agent(MockCompletionModel::from_turns([MockTurn::error(
            "provider unavailable",
        )])),
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"c"}"#,
        )),
    );
    let error = format!("{:#}", failed.run("question").await.unwrap_err());
    assert!(error.contains("CASPER"), "{error}");
}
