use rust_magi::agent::factory::MagiCoreID::{Balthasar, Casper, Melchior};
use rust_magi::bus::message::{AgentMessage, MessageBus};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn tasks_are_directed_and_responses_reach_peers_but_not_sender() {
    let mut bus = MessageBus::new(&[Balthasar, Casper, Melchior], 8).unwrap();
    let mut writer = bus.take_mailbox(Balthasar).unwrap();
    let mut reviewer = bus.take_mailbox(Casper).unwrap();
    assert_eq!(writer.id(), Balthasar);
    assert_eq!(reviewer.id(), Casper);
    let mut checker = bus.take_mailbox(Melchior).unwrap();

    bus.send(Balthasar, AgentMessage::Task("write".into()))
        .await
        .unwrap();
    assert_eq!(
        writer.recv().await,
        Some(AgentMessage::Task("write".into()))
    );

    writer.respond("article".into()).await.unwrap();
    let response = AgentMessage::Response(Balthasar, "article".into());
    assert_eq!(bus.recv().await.unwrap(), response);
    // A leaked Task would appear before the Response and fail these assertions.
    assert_eq!(reviewer.recv().await, Some(response.clone()));
    assert_eq!(checker.recv().await, Some(response));

    bus.shutdown().await.unwrap();
    // An echo to the writer would appear before Shutdown.
    for mailbox in [&mut writer, &mut reviewer, &mut checker] {
        assert_eq!(mailbox.recv().await, Some(AgentMessage::Shutdown));
    }
}

#[test]
fn invalid_topologies_and_mailbox_reuse_are_rejected() {
    assert!(MessageBus::new(&[Balthasar], 0).is_err());
    assert!(MessageBus::new(&[Balthasar, Balthasar], 8).is_err());
    assert!(MessageBus::new(&[], 8).is_err());
    let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
    assert!(bus.take_mailbox(Melchior).is_err());
    let _mailbox = bus.take_mailbox(Balthasar).unwrap();
    assert!(bus.take_mailbox(Balthasar).is_err());
}

#[tokio::test]
async fn unknown_and_closed_destinations_return_errors() {
    let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
    assert!(bus.send(Melchior, AgentMessage::Shutdown).await.is_err());
    drop(bus.take_mailbox(Balthasar).unwrap());
    assert!(bus.send(Balthasar, AgentMessage::Shutdown).await.is_err());
    assert!(bus.shutdown().await.is_err());
    assert!(
        timeout(Duration::from_secs(1), bus.recv())
            .await
            .unwrap()
            .is_err()
    );
}

#[tokio::test]
async fn a_closed_peer_is_reported_when_routing_a_response() {
    let mut bus = MessageBus::new(&[Balthasar, Casper], 8).unwrap();
    let a = bus.take_mailbox(Balthasar).unwrap();
    drop(bus.take_mailbox(Casper).unwrap());
    a.respond("article".into()).await.unwrap();
    let error = bus.recv().await.unwrap_err();
    assert!(error.to_string().contains("CASPER"));
}

#[tokio::test]
async fn dropping_bus_closes_mailboxes_and_response_channel() {
    let mut bus = MessageBus::new(&[Balthasar], 8).unwrap();
    let mut mailbox = bus.take_mailbox(Balthasar).unwrap();
    drop(bus);
    assert_eq!(mailbox.recv().await, None);
    assert!(mailbox.respond("late".into()).await.is_err());
}
