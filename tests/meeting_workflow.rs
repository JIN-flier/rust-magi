use std::time::Duration;

use rig::{
    prelude::*,
    test_utils::{MockCompletionModel, MockTurn},
};
use rust_magi::workflow::{meeting::MeetingWorkflow, result::WorkflowResult};

fn agent(turns: &[&str]) -> (Agent, MockCompletionModel) {
    let model = MockCompletionModel::from_turns(turns.iter().map(|text| MockTurn::text(*text)));
    (AgentBuilder::new(model.clone()).build(), model)
}

#[tokio::test]
async fn discussion_shares_every_initial_message_and_stops_on_unanimity() {
    let (balthasar, b_model) = agent(&[
        r#"{"message":"Use a staged rollout","proposal":"Use a staged rollout","agrees_with_candidate":false}"#,
        r#"{"message":"Agreed","proposal":"Use a staged rollout","agrees_with_candidate":true}"#,
    ]);
    let (casper, c_model) = agent(&[
        r#"{"message":"Keep it simple","proposal":"Keep it simple","agrees_with_candidate":false}"#,
        r#"{"message":"Agreed","proposal":"Use a staged rollout","agrees_with_candidate":true}"#,
    ]);
    let (melchior, m_model) = agent(&[
        r#"{"message":"Measure errors","proposal":"Measure errors","agrees_with_candidate":false}"#,
        r#"{"message":"Agreed","proposal":"Use a staged rollout","agrees_with_candidate":true}"#,
    ]);
    let result = MeetingWorkflow::new(balthasar, casper, melchior)
        .with_max_turns(3)
        .with_timeout(Duration::from_secs(3))
        .run("How should we release?")
        .await
        .unwrap();
    assert_eq!(result.consensus.as_deref(), Some("Use a staged rollout"));
    assert_eq!(result.rounds.len(), 2);
    let display = result.show();
    assert!(display.contains("共同意见：Use a staged rollout"));
    assert!(display.contains("第 2 轮"));
    for model in [&b_model, &c_model, &m_model] {
        assert_eq!(model.request_count(), 2);
        let requests = format!("{:?}", model.requests());
        for text in [
            "How should we release?",
            "Use a staged rollout",
            "Keep it simple",
            "Measure errors",
        ] {
            assert!(requests.contains(text), "missing {text}: {requests}");
        }
    }
}

#[tokio::test]
async fn max_turns_returns_unresolved_discussion() {
    let (balthasar, _) = agent(&[
        r#"{"message":"Plan A","proposal":"Plan A","agrees_with_candidate":false}"#,
        r#"{"message":"Plan A","proposal":"Plan A","agrees_with_candidate":true}"#,
    ]);
    let (casper, _) = agent(&[
        r#"{"message":"Plan B","proposal":"Plan B","agrees_with_candidate":false}"#,
        r#"{"message":"Plan B","proposal":"Plan B","agrees_with_candidate":false}"#,
    ]);
    let (melchior, _) = agent(&[
        r#"{"message":"Plan C","proposal":"Plan C","agrees_with_candidate":false}"#,
        r#"{"message":"Plan C","proposal":"Plan A","agrees_with_candidate":true}"#,
    ]);
    let result = MeetingWorkflow::new(balthasar, casper, melchior)
        .with_max_turns(2)
        .run("Choose a plan")
        .await
        .unwrap();
    assert_eq!(result.consensus, None);
    assert_eq!(result.rounds.len(), 2);
    assert!(result.show().contains("尚未取得三人共同意见"));
}

#[tokio::test]
async fn identical_initial_opinions_form_consensus_in_one_round() {
    let reply = r#"{"message":"Release gradually","proposal":"Release gradually","agrees_with_candidate":false}"#;
    let (a, _) = agent(&[reply]);
    let (b, _) = agent(&[reply]);
    let (c, _) = agent(&[reply]);
    let result = MeetingWorkflow::new(a, b, c)
        .with_max_turns(1)
        .run("How should we release?")
        .await
        .unwrap();
    assert_eq!(result.consensus.as_deref(), Some("Release gradually"));
    assert_eq!(result.rounds.len(), 1);
}

#[tokio::test]
async fn critique_without_a_standalone_proposal_is_rejected() {
    let (a, _) =
        agent(&[r#"{"message":"I disagree because of cost","agrees_with_candidate":false}"#]);
    let (b, _) =
        agent(&[r#"{"message":"Plan B","proposal":"Plan B","agrees_with_candidate":false}"#]);
    let (c, _) =
        agent(&[r#"{"message":"Plan C","proposal":"Plan C","agrees_with_candidate":false}"#]);
    let error = MeetingWorkflow::new(a, b, c)
        .with_max_turns(1)
        .run("Choose a plan")
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("proposal"));
}

#[tokio::test]
async fn dissenting_proposal_not_critique_becomes_next_candidate() {
    let (a, _) = agent(&[
        r#"{"message":"I suggest a full release","proposal":"Full release","agrees_with_candidate":false}"#,
        r#"{"message":"The full release works","proposal":"Full release","agrees_with_candidate":true}"#,
        r#"{"message":"I accept the pilot","proposal":"Pilot release","agrees_with_candidate":true}"#,
    ]);
    let (b, _) = agent(&[
        r#"{"message":"Start small","proposal":"Small release","agrees_with_candidate":false}"#,
        r#"{"message":"I disagree because rollout risk is high","proposal":"Pilot release","agrees_with_candidate":false}"#,
        r#"{"message":"A pilot works","proposal":"Pilot release","agrees_with_candidate":true}"#,
    ]);
    let (c, _) = agent(&[
        r#"{"message":"Need metrics","proposal":"Measured release","agrees_with_candidate":false}"#,
        r#"{"message":"The full release is acceptable","proposal":"Full release","agrees_with_candidate":true}"#,
        r#"{"message":"Pilot is acceptable","proposal":"Pilot release","agrees_with_candidate":true}"#,
    ]);
    let result = MeetingWorkflow::new(a, b, c)
        .with_max_turns(3)
        .run("How should we release?")
        .await
        .unwrap();
    assert_eq!(result.rounds[2].candidate.as_deref(), Some("Pilot release"));
    assert_eq!(result.consensus.as_deref(), Some("Pilot release"));
}

#[tokio::test]
async fn zero_turns_and_invalid_reply_are_errors() {
    let (a, _) = agent(&[]);
    let (b, _) = agent(&[]);
    let (c, _) = agent(&[]);
    assert!(
        MeetingWorkflow::new(a, b, c)
            .with_max_turns(0)
            .run("q")
            .await
            .is_err()
    );

    let (a, _) = agent(&["not json"]);
    let (b, _) = agent(&[r#"{"message":"B","proposal":"B","agrees_with_candidate":false}"#]);
    let (c, _) = agent(&[r#"{"message":"C","proposal":"C","agrees_with_candidate":false}"#]);
    assert!(MeetingWorkflow::new(a, b, c).run("q").await.is_err());
}

#[tokio::test]
async fn agreement_without_candidate_is_rejected() {
    let wrong = r#"{"message":"Agreed","proposal":"Plan A","agrees_with_candidate":true}"#;
    let valid = r#"{"message":"Propose","proposal":"Plan A","agrees_with_candidate":false}"#;
    let (a, _) = agent(&[wrong]);
    let (b, _) = agent(&[valid]);
    let (c, _) = agent(&[valid]);
    let error = MeetingWorkflow::new(a, b, c)
        .run("Choose a plan")
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("agreed without a meeting candidate"));
}

#[tokio::test]
async fn agreement_that_changes_candidate_is_rejected() {
    let first = r#"{"message":"Propose","proposal":"Plan A","agrees_with_candidate":false}"#;
    let dissent = r#"{"message":"Dissent","proposal":"Plan B","agrees_with_candidate":false}"#;
    let wrong = r#"{"message":"Agreed","proposal":"Plan C","agrees_with_candidate":true}"#;
    let valid = r#"{"message":"Agreed","proposal":"Plan A","agrees_with_candidate":true}"#;
    let (a, _) = agent(&[first, wrong]);
    let (b, _) = agent(&[dissent, valid]);
    let (c, _) = agent(&[first, valid]);
    let error = MeetingWorkflow::new(a, b, c)
        .with_max_turns(2)
        .run("Choose a plan")
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("agreed but changed the meeting candidate"));
}
