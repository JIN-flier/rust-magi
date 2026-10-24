use rust_magi::agent::factory::MagiCoreID;
use std::{future::pending, sync::Arc, time::Duration};

use rig::{
    agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext},
    prelude::*,
    test_utils::{MockCompletionModel, MockTurn},
};
use rust_magi::workflow::article_review::ArticleReviewWorkflow;
use tokio::{
    sync::{Barrier, Notify},
    time::timeout,
};

fn text_agent(text: &str) -> Agent {
    AgentBuilder::new(MockCompletionModel::text(text)).build()
}

#[tokio::test]
async fn three_agent_workflow_returns_outputs_and_passes_article_to_both_reviewers() {
    let writer = MockCompletionModel::text("ARTICLE: ownership moves values");
    let reviewer = MockCompletionModel::text("REVIEW: add examples");
    let checker = MockCompletionModel::text("FACT_CHECK: accurate");
    let workflow = ArticleReviewWorkflow::new(
        AgentBuilder::new(writer.clone()).build(),
        AgentBuilder::new(reviewer.clone()).build(),
        AgentBuilder::new(checker.clone()).build(),
    );
    let result = timeout(
        Duration::from_secs(3),
        workflow.run("Explain Rust ownership"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.article, "ARTICLE: ownership moves values");
    assert_eq!(result.review, "REVIEW: add examples");
    assert_eq!(result.fact_check, "FACT_CHECK: accurate");
    assert_eq!(writer.request_count(), 1);
    assert_eq!(reviewer.request_count(), 1);
    assert_eq!(checker.request_count(), 1);
    assert!(format!("{:?}", writer.requests()).contains("Explain Rust ownership"));
    for model in [&reviewer, &checker] {
        assert!(format!("{:?}", model.requests()).contains("ARTICLE: ownership moves values"));
    }
}

async fn assert_model_failure(failed_role: MagiCoreID) {
    let models: Vec<_> = [
        MagiCoreID::Balthasar,
        MagiCoreID::Casper,
        MagiCoreID::Melchior,
    ]
    .into_iter()
    .map(|id| {
        if id == failed_role {
            MockCompletionModel::from_turns([MockTurn::error("provider unavailable")])
        } else {
            MockCompletionModel::text("ok")
        }
    })
    .collect();
    let workflow = ArticleReviewWorkflow::new(
        AgentBuilder::new(models[0].clone()).build(),
        AgentBuilder::new(models[1].clone()).build(),
        AgentBuilder::new(models[2].clone()).build(),
    );
    let error = timeout(Duration::from_secs(3), workflow.run("task"))
        .await
        .expect("a model failure must not hang")
        .unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("provider unavailable"), "{error}");
    let expected_agent = match failed_role {
        MagiCoreID::Balthasar => "Agent BALTHASAR",
        MagiCoreID::Casper => "Agent CASPER",
        MagiCoreID::Melchior => "Agent MELCHIOR",
    };
    assert!(error.contains(expected_agent), "{error}");
    if failed_role == MagiCoreID::Balthasar {
        assert_eq!(models[1].request_count(), 0);
        assert_eq!(models[2].request_count(), 0);
    }
}

#[tokio::test]
async fn writer_failure_stops_before_review() {
    assert_model_failure(MagiCoreID::Balthasar).await;
}

#[tokio::test]
async fn reviewer_failure_returns_error() {
    assert_model_failure(MagiCoreID::Casper).await;
}

#[tokio::test]
async fn fact_checker_failure_returns_error() {
    assert_model_failure(MagiCoreID::Melchior).await;
}

struct Rendezvous(Arc<Barrier>);

impl AgentHook for Rendezvous {
    async fn on_completion_call(
        &self,
        _: &HookContext,
        _: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        self.0.wait().await;
        CompletionCallAction::continue_run()
    }
}

#[tokio::test]
async fn reviewers_run_concurrently() {
    // Neither reviewer can finish until both have started. Sequential dispatch hangs.
    let barrier = Arc::new(Barrier::new(2));
    let workflow = ArticleReviewWorkflow::new(
        text_agent("article"),
        AgentBuilder::new(MockCompletionModel::text("review"))
            .add_hook(Rendezvous(barrier.clone()))
            .build(),
        AgentBuilder::new(MockCompletionModel::text("facts"))
            .add_hook(Rendezvous(barrier))
            .build(),
    );
    let result = timeout(Duration::from_secs(3), workflow.run("task"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.review, "review");
    assert_eq!(result.fact_check, "facts");
}

struct NotifyOnDrop(Arc<Notify>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

struct BlockRequest {
    started: Arc<Notify>,
    cancelled: Arc<Notify>,
    barrier: Option<Arc<Barrier>>,
}

impl AgentHook for BlockRequest {
    async fn on_completion_call(
        &self,
        _: &HookContext,
        _: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        let _guard = NotifyOnDrop(self.cancelled.clone());
        self.started.notify_one();
        if let Some(barrier) = &self.barrier {
            barrier.wait().await;
        }
        pending().await
    }
}

#[tokio::test]
async fn timeout_cancels_in_flight_request() {
    let cancelled = Arc::new(Notify::new());
    let writer = AgentBuilder::new(MockCompletionModel::text("unused"))
        .add_hook(BlockRequest {
            started: Arc::new(Notify::new()),
            cancelled: cancelled.clone(),
            barrier: None,
        })
        .build();
    let workflow = ArticleReviewWorkflow::new(writer, text_agent("review"), text_agent("facts"))
        .with_timeout(Duration::from_secs(1));
    let error = timeout(Duration::from_secs(3), workflow.run("task"))
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("timed out"), "{error:#}");
    timeout(Duration::from_secs(1), cancelled.notified())
        .await
        .expect("request must be dropped");
}

#[tokio::test]
async fn cancelling_workflow_cancels_its_agents() {
    let started = Arc::new(Notify::new());
    let cancelled = Arc::new(Notify::new());
    let writer = AgentBuilder::new(MockCompletionModel::text("unused"))
        .add_hook(BlockRequest {
            started: started.clone(),
            cancelled: cancelled.clone(),
            barrier: None,
        })
        .build();
    let workflow = ArticleReviewWorkflow::new(writer, text_agent("review"), text_agent("facts"));
    let task = tokio::spawn(async move { workflow.run("task").await });
    timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    timeout(Duration::from_secs(1), cancelled.notified())
        .await
        .expect("no detached model request");
}

#[tokio::test]
async fn failing_reviewer_cancels_the_other_review_request() {
    let barrier = Arc::new(Barrier::new(2));
    let cancelled = Arc::new(Notify::new());
    let reviewer = AgentBuilder::new(MockCompletionModel::text("unused"))
        .add_hook(BlockRequest {
            started: Arc::new(Notify::new()),
            cancelled: cancelled.clone(),
            barrier: Some(barrier.clone()),
        })
        .build();
    let checker = AgentBuilder::new(MockCompletionModel::from_turns([MockTurn::error(
        "checker failed",
    )]))
    .add_hook(Rendezvous(barrier))
    .build();
    let workflow = ArticleReviewWorkflow::new(text_agent("article"), reviewer, checker);
    let error = timeout(Duration::from_secs(3), workflow.run("task"))
        .await
        .unwrap()
        .unwrap_err();
    assert!(format!("{error:#}").contains("checker failed"));
    timeout(Duration::from_secs(1), cancelled.notified())
        .await
        .expect("other reviewer must be cancelled");
}

struct PanicRequest;

impl AgentHook for PanicRequest {
    async fn on_completion_call(
        &self,
        _: &HookContext,
        _: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        panic!("model panicked")
    }
}

#[tokio::test]
async fn agent_panic_returns_error_instead_of_hanging() {
    let writer = AgentBuilder::new(MockCompletionModel::text("unused"))
        .add_hook(PanicRequest)
        .build();
    let workflow = ArticleReviewWorkflow::new(writer, text_agent("review"), text_agent("facts"));
    let error = timeout(Duration::from_secs(3), workflow.run("task"))
        .await
        .unwrap()
        .unwrap_err();
    assert!(format!("{error:#}").contains("model panicked"));
}
