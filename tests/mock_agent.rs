use rig::ProviderResponseError;
use rig::message::AssistantContent;
use rig::providers::openai::extension::Prediction::Content;
use rig::test_utils::{MockError, MockTurn};
use rig::{prelude::*, test_utils::MockCompletionModel};
use rust_magi::agent::factory::MagiCoreID::{Balthasar, Casper};
use rust_magi::{
    agent::atuonomous::AutonomousAgent,
    bus::message::{AgentMessage, MessageBus},
};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn task_and_trigger_produce_responses_but_peer_responses_do_not() {
    for message in [
        AgentMessage::Task("task".into()),
        AgentMessage::Trigger("trigger".into()),
    ] {
        let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
        let model = MockCompletionModel::text("answer");
        let agent = AutonomousAgent::new(
            AgentBuilder::new(model.clone()).build(),
            bus.take_mailbox(Balthasar).unwrap(),
        );
        bus.send(Balthasar, AgentMessage::Response(Casper, "context".into()))
            .await
            .unwrap();
        bus.send(Balthasar, message).await.unwrap();
        let task = tokio::spawn(agent.run());
        assert_eq!(
            timeout(Duration::from_secs(1), bus.recv())
                .await
                .unwrap()
                .unwrap(),
            AgentMessage::Response(Balthasar, "answer".into())
        );
        bus.shutdown().await.unwrap();
        timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(model.request_count(), 1);
    }
}

#[tokio::test]
async fn agent_exits_when_bus_closes() {
    let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
    let model = MockCompletionModel::text("unused");
    let agent = AutonomousAgent::new(
        AgentBuilder::new(model.clone()).build(),
        bus.take_mailbox(Balthasar).unwrap(),
    );
    drop(bus);
    timeout(Duration::from_secs(1), agent.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(model.request_count(), 0);
}

#[tokio::test]
async fn error_message_terminates_agent_with_context() {
    let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
    let agent = AutonomousAgent::new(
        AgentBuilder::new(MockCompletionModel::text("unused")).build(),
        bus.take_mailbox(Balthasar).unwrap(),
    );
    bus.send(Balthasar, AgentMessage::Error("stop reason".into()))
        .await
        .unwrap();
    let error = timeout(Duration::from_secs(1), agent.run())
        .await
        .unwrap()
        .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("BALTHASAR"));
    assert!(message.contains("stop reason"));
}

