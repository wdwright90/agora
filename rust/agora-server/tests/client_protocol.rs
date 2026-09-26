//! Server-side acceptance tests for SPEC-002 (client protocol). Test names start with the
//! requirement ID they verify.

mod common;

use agora_protocol::{
    AgentId, ClientMessage, ErrorCode, PROTOCOL_VERSION, Placement, ProtocolVersion, RunId,
    RunPhase, ServerMessage, StateId,
};
use agora_server::RETAINED_RESULTS;
use common::{
    Client, cell, empty_grid, expect_code, expect_error, request_id, start_default_server,
};
use serde_json::json;

fn agent(n: u64) -> AgentId {
    AgentId::new(n).unwrap()
}

#[tokio::test]
async fn r01_unknown_message_types_are_rejected_with_their_type() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let reply = client
        .exchange_text(r#"{ "type": "dance", "request_id": 3 }"#)
        .await;

    let error = expect_code(reply, ErrorCode::UnknownMessageType);
    assert_eq!(error.request_id, Some(request_id(3)));
    assert_eq!(error.details.unwrap()["type"], "dance");
}

#[tokio::test]
async fn r01_invalid_frames_are_malformed_and_echo_a_readable_request_id() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    for text in ["not json", "[1, 2]", r#"{ "request_id": 4 }"#] {
        let error = expect_code(
            client.exchange_text(text).await,
            ErrorCode::MalformedMessage,
        );
        let expected = text.contains("request_id").then(|| request_id(4));
        assert_eq!(error.request_id, expected, "{text}");
    }

    // A known type with a missing field still echoes its request ID.
    let reply = client
        .exchange_text(r#"{ "type": "spawn", "request_id": 2 }"#)
        .await;
    let error = expect_code(reply, ErrorCode::MalformedMessage);
    assert_eq!(error.request_id, Some(request_id(2)));

    client.send_binary(b"{}".to_vec()).await;
    let error = expect_code(client.receive().await, ErrorCode::MalformedMessage);
    assert_eq!(error.request_id, None);
}

#[tokio::test]
async fn r02_unknown_fields_in_requests_are_ignored() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let reply = client
        .exchange_text(
            r#"{ "type": "create_run", "request_id": 1, "catalog_entry": "empty-grid-10x10", "later": true }"#,
        )
        .await;

    assert!(
        matches!(reply, ServerMessage::RunCreated { .. }),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r04_hello_is_answered_with_the_server_version() {
    let address = start_default_server().await;
    let mut client = Client::connect(address).await;

    let reply = client
        .exchange(&ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;

    assert_eq!(
        reply,
        ServerMessage::Welcome {
            protocol_version: PROTOCOL_VERSION
        }
    );
}

#[tokio::test]
async fn r04_r06_an_unsupported_version_is_rejected_and_the_connection_closed() {
    let address = start_default_server().await;
    let mut client = Client::connect(address).await;

    let reply = client
        .exchange(&ClientMessage::Hello {
            protocol_version: ProtocolVersion::new(0, 2, 0),
        })
        .await;

    let error = expect_code(reply, ErrorCode::UnsupportedProtocolVersion);
    assert_eq!(error.request_id, None);
    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(
        details,
        json!({ "requested": "0.2.0", "supported": [PROTOCOL_VERSION.to_string()] })
    );
    client.expect_closed().await;
}

#[tokio::test]
async fn r04_requests_before_hello_are_rejected() {
    let address = start_default_server().await;
    let mut client = Client::connect(address).await;

    let reply = client
        .exchange(&ClientMessage::CreateRun {
            request_id: request_id(1),
            catalog_entry: empty_grid(),
        })
        .await;
    let error = expect_code(reply, ErrorCode::HandshakeRequired);
    assert_eq!(error.request_id, Some(request_id(1)));

    // The connection stays open, and the handshake can still complete.
    let reply = client
        .exchange(&ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;
    assert!(matches!(reply, ServerMessage::Welcome { .. }), "{reply:?}");
}

#[tokio::test]
async fn r04_a_second_hello_is_malformed() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let reply = client
        .exchange(&ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;

    let error = expect_code(reply, ErrorCode::MalformedMessage);
    assert_eq!(error.request_id, None);
}

#[tokio::test]
async fn r07_a_retry_returns_the_original_result_without_executing_again() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let spawn = ClientMessage::Spawn {
        request_id: request_id(2),
        placement: Placement::Random,
    };

    let first = client.exchange(&spawn).await;
    let retry = client.exchange(&spawn).await;

    assert_eq!(
        first,
        ServerMessage::Spawned {
            request_id: request_id(2),
            agent_id: agent(1)
        }
    );
    assert_eq!(retry, first);
    // The retry spawned nothing: the next agent is 2.
    client.next_id();
    let reply = client.spawn(Placement::Random).await;
    assert!(
        matches!(reply, ServerMessage::Spawned { agent_id, .. } if agent_id == agent(2)),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r07_a_retried_create_run_returns_the_same_run() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;
    let create = ClientMessage::CreateRun {
        request_id: request_id(1),
        catalog_entry: empty_grid(),
    };

    let first = client.exchange(&create).await;
    let retry = client.exchange(&create).await;

    assert!(
        matches!(first, ServerMessage::RunCreated { .. }),
        "{first:?}"
    );
    assert_eq!(retry, first);
}

#[tokio::test]
async fn r07_error_results_are_retained_too() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let spawn = ClientMessage::Spawn {
        request_id: request_id(2),
        placement: cell(10, 0),
    };

    let first = client.exchange(&spawn).await;
    let retry = client.exchange(&spawn).await;

    expect_code(first.clone(), ErrorCode::CellOutOfBounds);
    assert_eq!(retry, first);
}

#[tokio::test]
async fn r07_reusing_an_id_for_a_different_request_conflicts() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    client
        .exchange(&ClientMessage::Spawn {
            request_id: request_id(2),
            placement: cell(0, 0),
        })
        .await;

    let reply = client
        .exchange(&ClientMessage::Spawn {
            request_id: request_id(2),
            placement: cell(1, 1),
        })
        .await;

    let error = expect_code(reply, ErrorCode::RequestIdConflict);
    assert_eq!(error.request_id, Some(request_id(2)));
}

#[tokio::test]
async fn r07_a_skipped_id_within_the_retained_results_is_stale() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    client
        .exchange(&ClientMessage::Spawn {
            request_id: request_id(3),
            placement: Placement::Random,
        })
        .await;

    let reply = client
        .exchange(&ClientMessage::Spawn {
            request_id: request_id(2),
            placement: Placement::Random,
        })
        .await;

    let error = expect_code(reply, ErrorCode::StaleRequestId);
    assert_eq!(error.details.unwrap()["highest_admitted"], 3);
}

#[tokio::test]
async fn r07_ids_older_than_the_retained_results_have_expired() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;
    let create = ClientMessage::CreateRun {
        request_id: client.next_id(),
        catalog_entry: empty_grid(),
    };
    client.exchange(&create).await;
    let spawns: Vec<ClientMessage> = (0..RETAINED_RESULTS)
        .map(|_| ClientMessage::Spawn {
            request_id: client.next_id(),
            placement: Placement::Random,
        })
        .collect();
    for spawn in &spawns {
        client.exchange(spawn).await;
    }

    // The create_run result was the oldest of RETAINED_RESULTS + 1 results.
    let error = expect_code(client.exchange(&create).await, ErrorCode::ResultExpired);
    assert_eq!(error.request_id, Some(request_id(1)));
    // The last RETAINED_RESULTS results are still replayed.
    let reply = client.exchange(&spawns[0]).await;
    assert!(
        matches!(reply, ServerMessage::Spawned { agent_id, .. } if agent_id == agent(1)),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r07_different_sessions_use_request_ids_independently() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    // Both sessions have used request 1; each now uses request 2 for a different spawn.
    let first = creator.spawn(cell(0, 0)).await;
    let second = joiner.spawn(cell(1, 0)).await;

    assert!(matches!(first, ServerMessage::Spawned { .. }), "{first:?}");
    assert!(
        matches!(second, ServerMessage::Spawned { .. }),
        "{second:?}"
    );
}

#[tokio::test]
async fn r08_create_run_returns_a_new_run_in_setup_at_state_0() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let reply = client
        .exchange(&ClientMessage::CreateRun {
            request_id: request_id(1),
            catalog_entry: empty_grid(),
        })
        .await;

    let ServerMessage::RunCreated {
        request_id: echoed,
        catalog_entry,
        state_id,
        phase,
        ..
    } = reply
    else {
        panic!("expected run_created, got {reply:?}");
    };
    assert_eq!(echoed, request_id(1));
    assert_eq!(catalog_entry, empty_grid());
    assert_eq!(state_id, StateId::new(0).unwrap());
    assert_eq!(phase, RunPhase::Setup);
}

#[tokio::test]
async fn r08_join_run_establishes_a_new_session_and_reports_the_phase() {
    let address = start_default_server().await;
    let mut creator = Client::connect_with_hello(address).await;
    let request_id = creator.next_id();
    let ServerMessage::RunCreated {
        run_id, session_id, ..
    } = creator
        .exchange(&ClientMessage::CreateRun {
            request_id,
            catalog_entry: empty_grid(),
        })
        .await
    else {
        panic!("expected run_created");
    };

    let mut early = Client::connect_with_hello(address).await;
    let reply = early.join_run(&run_id).await;
    let ServerMessage::RunJoined {
        run_id: joined_run,
        session_id: joined_session,
        catalog_entry,
        state_id,
        phase,
        ..
    } = reply
    else {
        panic!("expected run_joined, got {reply:?}");
    };
    assert_eq!(joined_run, run_id);
    assert_ne!(joined_session, session_id);
    assert_eq!(catalog_entry, empty_grid());
    assert_eq!(state_id, StateId::new(0).unwrap());
    assert_eq!(phase, RunPhase::Setup);

    creator.spawn(Placement::Random).await;
    creator.start().await;
    let mut late = Client::connect_with_hello(address).await;
    let reply = late.join_run(&run_id).await;
    assert!(
        matches!(
            reply,
            ServerMessage::RunJoined {
                phase: RunPhase::Started,
                ..
            }
        ),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r08_spawn_assigns_agent_ids_across_sessions() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    let first = creator.spawn(cell(0, 0)).await;
    let second = joiner.spawn(cell(9, 9)).await;

    assert_eq!(
        first,
        ServerMessage::Spawned {
            request_id: request_id(2),
            agent_id: agent(1)
        }
    );
    assert_eq!(
        second,
        ServerMessage::Spawned {
            request_id: request_id(2),
            agent_id: agent(2)
        }
    );
}

#[tokio::test]
async fn r08_start_confirms_the_run_started() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    client.spawn(Placement::Random).await;

    let reply = client.start().await;

    assert_eq!(
        reply,
        ServerMessage::Started {
            request_id: request_id(3)
        }
    );
}

#[tokio::test]
async fn r08_a_connection_has_at_most_one_session() {
    let address = start_default_server().await;
    let (mut client, run_id) = Client::with_new_run(address).await;

    let request_id = client.next_id();
    let create = client
        .exchange(&ClientMessage::CreateRun {
            request_id,
            catalog_entry: empty_grid(),
        })
        .await;
    let join = client.join_run(&run_id).await;

    expect_code(create, ErrorCode::SessionAlreadyEstablished);
    expect_code(join, ErrorCode::SessionAlreadyEstablished);
}

#[tokio::test]
async fn r08_run_scoped_requests_need_a_session() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let spawn = client.spawn(Placement::Random).await;
    let start = client.start().await;

    let error = expect_code(spawn, ErrorCode::NoSession);
    assert_eq!(error.request_id, Some(request_id(1)));
    expect_code(start, ErrorCode::NoSession);
}

#[tokio::test]
async fn r09_unknown_catalog_entries_and_runs_are_reported_with_details() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let reply = client
        .exchange_text(r#"{ "type": "create_run", "request_id": 1, "catalog_entry": "maze" }"#)
        .await;
    let error = expect_code(reply, ErrorCode::UnknownCatalogEntry);
    assert_eq!(error.details.unwrap()["catalog_entry"], "maze");

    let reply = client.join_run(&RunId::new("no-such-run").unwrap()).await;
    let error = expect_code(reply, ErrorCode::UnknownRun);
    assert_eq!(error.details.unwrap()["run_id"], "no-such-run");
}

#[tokio::test]
async fn r09_spawn_failures_map_to_their_codes() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;

    let error = expect_code(client.spawn(cell(3, 10)).await, ErrorCode::CellOutOfBounds);
    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(details, json!({ "x": 3, "y": 10 }));

    client.spawn(cell(2, 2)).await;
    let error = expect_code(client.spawn(cell(2, 2)).await, ErrorCode::CellOccupied);
    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(details, json!({ "x": 2, "y": 2 }));

    for _ in 1..100 {
        let reply = client.spawn(Placement::Random).await;
        assert!(matches!(reply, ServerMessage::Spawned { .. }), "{reply:?}");
    }
    expect_code(client.spawn(Placement::Random).await, ErrorCode::NoFreeCell);
}

#[tokio::test]
async fn r09_errors_carry_a_message_for_people() {
    let address = start_default_server().await;
    let mut client = Client::connect_with_hello(address).await;

    let error = expect_error(client.start().await);

    assert!(!error.message.is_empty());
}
