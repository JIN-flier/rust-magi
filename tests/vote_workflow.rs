use std::{future::pending, sync::Arc, time::Duration};

use rig::{
    agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext},
    prelude::*,
    test_utils::{MockCompletionModel, MockTurn},
};
use rust_magi::{
    agent::{factory::MagiCoreID, intent::Intention},
    workflow::{result::WorkflowResult, vote::VoteWorkflow},
};
use tokio::{sync::Notify, time::timeout};

fn choices() -> Intention {
    Intention::Vote {
        option_a: "Proceed with rollout".into(),
        option_b: "Delay rollout".into(),
    }
}

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
    .run("Should we proceed?", &choices())
    .await
    .unwrap();

    assert!(result.decision);
    assert_eq!(result.votes.len(), 3);
    assert_eq!(result.votes[0].id, MagiCoreID::Balthasar);
    assert_eq!(result.votes[1].id, MagiCoreID::Casper);
    assert_eq!(result.votes[2].id, MagiCoreID::Melchior);
    assert_eq!(result.votes[1].reason, "too costly");
    assert!(result.show().contains("多数决：true"));
    for model in [&balthasar, &casper, &melchior] {
        assert_eq!(model.request_count(), 1);
        let requests = format!("{:?}", model.requests());
        for text in [
            "Should we proceed?",
            "Proceed with rollout",
            "Delay rollout",
        ] {
            assert!(requests.contains(text), "missing {text}: {requests}");
        }
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
    .run("question", &choices())
    .await
    .unwrap();
    assert!(!result.decision);
    assert!(result.show().contains("多数决：false"));
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
    assert!(invalid.run("question", &choices()).await.is_err());

    let failed = VoteWorkflow::new(
        agent(MockCompletionModel::text(r#"{"choice":true,"reason":"a"}"#)),
        agent(MockCompletionModel::from_turns([MockTurn::error(
            "provider unavailable",
        )])),
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"c"}"#,
        )),
    );
    let error = format!(
        "{:#}",
        failed.run("question", &choices()).await.unwrap_err()
    );
    assert!(error.contains("CASPER"), "{error}");
}

#[tokio::test]
async fn blank_reason_is_rejected() {
    let workflow = VoteWorkflow::new(
        agent(MockCompletionModel::text(
            r#"{"choice":true,"reason":"ok"}"#,
        )),
        agent(MockCompletionModel::text(
            r#"{"choice":false,"reason":"  "}"#,
        )),
        agent(MockCompletionModel::text(
            r#"{"choice":true,"reason":"ok"}"#,
        )),
    );
    let error = format!(
        "{:#}",
        workflow.run("question", &choices()).await.unwrap_err()
    );
    assert!(error.contains("CASPER"), "{error}");
    assert!(error.contains("empty vote reason"), "{error}");
}

struct SignalOnDrop(Arc<Notify>);

impl Drop for SignalOnDrop {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

struct BlockCompletion {
    started: Arc<Notify>,
    cancelled: Arc<Notify>,
}

impl AgentHook for BlockCompletion {
    async fn on_completion_call(
        &self,
        _: &HookContext,
        _: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        let _guard = SignalOnDrop(self.cancelled.clone());
        self.started.notify_one();
        pending().await
    }
}

#[tokio::test]
async fn timeout_cancels_in_flight_vote_request() {
    let started = Arc::new(Notify::new());
    let cancelled = Arc::new(Notify::new());
    let blocked = AgentBuilder::new(MockCompletionModel::text("unused"))
        .add_hook(BlockCompletion {
            started: started.clone(),
            cancelled: cancelled.clone(),
        })
        .build();
    let valid = || {
        agent(MockCompletionModel::text(
            r#"{"choice":true,"reason":"ok"}"#,
        ))
    };
    let workflow =
        VoteWorkflow::new(blocked, valid(), valid()).with_timeout(Duration::from_millis(100));
    let result = timeout(Duration::from_secs(2), workflow.run("question", &choices()))
        .await
        .expect("workflow timeout must finish");
    assert!(result.unwrap_err().to_string().contains("timed out"));
    timeout(Duration::from_secs(1), started.notified())
        .await
        .expect("blocked request must have started");
    timeout(Duration::from_secs(1), cancelled.notified())
        .await
        .expect("blocked request must be dropped");
}
