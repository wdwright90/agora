//! Server-side acceptance tests for SPEC-002 (client protocol). Test names start with the
//! requirement ID they verify.

mod common;

use agora_protocol::{
    AgentId, AgentView, ClientMessage, CloseReason, Direction, ErrorCode, IntervalMs, KindId,
    PROTOCOL_VERSION, Pacing, PacingMode, Placement, ProtocolVersion, RunId, RunPhase,
    ServerMessage, StateId, View,
};
use agora_server::RETAINED_RESULTS;
use common::{
    Client, cell, empty_grid, expect_code, expect_error, expect_spawned, expect_submitted,
    request_id, start_default_server, state, stay, step,
};
use serde_json::json;

fn agent(n: u64) -> AgentId {
    AgentId::new(n).unwrap()
}

fn agent_kind() -> KindId {
    KindId::new("agent").unwrap()
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
            protocol_version: ProtocolVersion::new(0, 1, 0),
        })
        .await;

    let error = expect_code(reply, ErrorCode::UnsupportedProtocolVersion);
    assert_eq!(error.request_id, None);
    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(
        details,
        json!({ "requested": "0.1.0", "supported": [PROTOCOL_VERSION.to_string()] })
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

    let (mut divided, _) = Client::with_new_run_from(address, common::divided()).await;
    let error = expect_code(divided.spawn(cell(5, 0)).await, ErrorCode::CellBlocked);
    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(details, json!({ "x": 5, "y": 0 }));

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

#[tokio::test]
async fn r10_submitted_reports_each_entry_in_order() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let first = expect_spawned(client.spawn(cell(0, 0)).await);
    let second = expect_spawned(client.spawn(cell(5, 5)).await);
    client.start().await;
    let unknown = agent(99);

    let reply = client
        .submit(
            state(0),
            vec![
                step(first, Direction::North, 2),
                stay(second),
                step(first, Direction::North, 1),
                stay(first),
                stay(unknown),
            ],
        )
        .await;

    let ServerMessage::Submitted {
        state_id, results, ..
    } = reply
    else {
        panic!("expected submitted, got {reply:?}");
    };
    assert_eq!(state_id, state(0));
    let summary: Vec<_> = results
        .iter()
        .map(|r| (r.agent_id, r.error.as_ref().map(|e| e.code.clone())))
        .collect();
    assert_eq!(
        summary,
        [
            (first, Some(ErrorCode::DistanceBudgetExceeded)),
            (second, None),
            (first, None),
            (first, Some(ErrorCode::AlreadySubmitted)),
            (unknown, Some(ErrorCode::AgentNotOwned)),
        ]
    );
    let details = serde_json::Value::Object(results[0].error.clone().unwrap().details.unwrap());
    assert_eq!(details, json!({ "distance": 2, "budget": 1 }));
}

#[tokio::test]
async fn r10_submitting_before_start_is_rejected() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let first = expect_spawned(client.spawn(Placement::Random).await);

    expect_code(
        client.submit(state(0), vec![stay(first)]).await,
        ErrorCode::RunNotStarted,
    );
}

#[tokio::test]
async fn r10_submitting_for_another_state_is_rejected() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let first = expect_spawned(client.spawn(Placement::Random).await);
    client.start().await;

    let error = expect_code(
        client.submit(state(1), vec![stay(first)]).await,
        ErrorCode::WrongState,
    );

    let details = serde_json::Value::Object(error.details.unwrap());
    assert_eq!(details, json!({ "expected": 0, "supplied": 1 }));
    // The rejected submission did not fill the agent's slot.
    let results = expect_submitted(client.submit(state(0), vec![stay(first)]).await);
    assert_eq!(results[0].error, None);
}

#[tokio::test]
async fn r11_observations_follow_the_response_that_caused_them() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let first = expect_spawned(client.spawn(Placement::Random).await);
    let second = expect_spawned(client.spawn(Placement::Random).await);

    let request_id = client.next_id();
    client.send(&ClientMessage::Start { request_id }).await;
    assert_eq!(
        client.receive_raw().await,
        ServerMessage::Started { request_id }
    );
    assert_eq!(client.observations().await, (state(0), vec![first, second]));

    client.submit(state(0), vec![stay(second)]).await;
    let request_id = client.next_id();
    client
        .send(&ClientMessage::Submit {
            request_id,
            state_id: state(0),
            actions: vec![stay(first)],
        })
        .await;
    let reply = client.receive_raw().await;
    assert!(
        matches!(reply, ServerMessage::Submitted { .. }),
        "{reply:?}"
    );
    assert_eq!(client.observations().await, (state(1), vec![first, second]));
}

#[tokio::test]
async fn r12_watching_returns_the_current_view() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;
    let first = expect_spawned(client.spawn(cell(0, 9)).await);

    let view = client.watch().await;

    assert_eq!(
        view,
        View {
            state_id: state(0),
            phase: RunPhase::Setup,
            width: 10,
            height: 10,
            agents: vec![AgentView {
                agent_id: first,
                kind: agent_kind(),
                x: 0,
                y: 9
            }],
        }
    );
}

#[tokio::test]
async fn r12_watching_carries_the_run_kinds() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;

    let kinds = client.watching().await.kinds;

    assert_eq!(
        serde_json::to_value(kinds).unwrap(),
        json!([
            { "category": "terrain", "kind": "floor", "class": "floor",
              "blocks_movement": false, "blocks_sight": false },
            { "category": "terrain", "kind": "wall", "class": "wall",
              "blocks_movement": true, "blocks_sight": true },
            { "category": "item", "kind": "berry",
              "appearance": { "hue": 0.02, "size": 0.3, "shape": "round" } },
            { "category": "creature", "kind": "agent",
              "appearance": { "hue": 0.6, "size": 0.5, "shape": "agent" } },
        ])
    );
}

#[tokio::test]
async fn r12_watching_carries_the_terrain_as_kind_indices() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run_from(address, common::divided()).await;

    let watching = client.watching().await;

    // Built-in kinds: floor is index 0 and wall is index 1. Cells are row-major from the
    // south-west corner.
    let expected: Vec<u32> = (0..10)
        .flat_map(|y| (0..10).map(move |x| u32::from(x == 5 && !(4..=5).contains(&y))))
        .collect();
    assert_eq!(watching.terrain, expected);
    assert_eq!(watching.kinds[1].id().as_str(), "wall");

    let (mut open, _) = Client::with_new_run(address).await;
    assert_eq!(open.watching().await.terrain, vec![0; 100]);
}

#[tokio::test]
async fn r12_viewers_receive_a_view_after_each_change() {
    let address = start_default_server().await;
    let (mut viewer, run_id) = Client::with_new_run(address).await;
    let mut player = Client::joined(address, &run_id).await;
    viewer.watch().await;

    let first = expect_spawned(player.spawn(cell(4, 4)).await);
    let view = viewer.view_update().await;
    assert_eq!((view.state_id, view.phase), (state(0), RunPhase::Setup));
    assert_eq!(view.agents.len(), 1);

    viewer.start().await;
    let view = viewer.view_update().await;
    assert_eq!((view.state_id, view.phase), (state(0), RunPhase::Started));

    player
        .submit(state(0), vec![step(first, Direction::East, 1)])
        .await;
    let view = viewer.view_update().await;
    assert_eq!(view.state_id, state(1));
    assert_eq!(
        view.agents,
        [AgentView {
            agent_id: first,
            kind: agent_kind(),
            x: 5,
            y: 4
        }]
    );
}

#[tokio::test]
async fn r13_pacing_requests_receive_their_responses_and_updates() {
    let address = start_default_server().await;
    let (mut viewer, _) = Client::with_new_run(address).await;

    let (_, pacing) = viewer.watch_with_pacing().await;
    assert_eq!(
        pacing,
        Pacing {
            mode: PacingMode::Interval {
                ms: IntervalMs::new(500).unwrap()
            },
            you_control: false
        }
    );

    let reply = viewer.claim_pacing().await;
    assert!(
        matches!(reply, ServerMessage::PacingClaimed { .. }),
        "{reply:?}"
    );
    assert!(viewer.pacing_update().await.you_control);

    let reply = viewer.set_pacing(PacingMode::Paused).await;
    assert!(
        matches!(reply, ServerMessage::PacingSet { .. }),
        "{reply:?}"
    );
    assert_eq!(viewer.pacing_update().await.mode, PacingMode::Paused);

    let reply = viewer.step_once().await;
    assert!(
        matches!(reply, ServerMessage::StepGranted { .. }),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r13_pacing_requests_report_their_errors() {
    let address = start_default_server().await;
    let (mut first, run_id) = Client::with_new_run(address).await;
    let mut second = Client::joined(address, &run_id).await;

    expect_code(first.claim_pacing().await, ErrorCode::NotViewing);
    first.watch_and_control().await;
    second.watch().await;
    expect_code(second.claim_pacing().await, ErrorCode::PacingControlHeld);
    expect_code(
        second.set_pacing(PacingMode::Unlimited).await,
        ErrorCode::NotPacingController,
    );
    expect_code(second.step_once().await, ErrorCode::NotPacingController);
    expect_code(first.step_once().await, ErrorCode::RunNotPaused);
}

#[tokio::test]
async fn r14_leaving_ends_the_session_and_the_connection_can_start_another() {
    let address = start_default_server().await;
    let (mut client, run_id) = Client::with_new_run(address).await;
    let mut other = Client::joined(address, &run_id).await;

    let reply = other.leave_run().await;
    assert!(matches!(reply, ServerMessage::Left { .. }), "{reply:?}");

    // The session has ended: run requests, including a retried leave, have no session.
    expect_code(other.spawn(Placement::Random).await, ErrorCode::NoSession);
    expect_code(other.leave_run().await, ErrorCode::NoSession);
    // The same connection can establish another session.
    let reply = other.join_run(&run_id).await;
    assert!(
        matches!(reply, ServerMessage::RunJoined { .. }),
        "{reply:?}"
    );
    let reply = client.spawn(Placement::Random).await;
    assert!(matches!(reply, ServerMessage::Spawned { .. }), "{reply:?}");
}

#[tokio::test]
async fn r14_only_the_creator_can_close_the_run() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    expect_code(joiner.close_run().await, ErrorCode::NotCreator);

    let reply = creator.close_run().await;
    assert!(matches!(reply, ServerMessage::Closed { .. }), "{reply:?}");
    expect_code(creator.start().await, ErrorCode::NoSession);
}

#[tokio::test]
async fn r15_run_closed_names_the_reason_and_ends_the_session() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    creator.close_run().await;

    assert_eq!(
        joiner.receive().await,
        ServerMessage::RunClosed {
            reason: CloseReason::ClosedByCreator
        }
    );
    expect_code(joiner.spawn(Placement::Random).await, ErrorCode::NoSession);
    // The connection can create a run of its own.
    joiner.create_run().await;
}
