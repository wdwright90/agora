//! Acceptance tests for SPEC-002 (client protocol). Test names start with the requirement ID
//! they verify. Shared fixtures live in `docs/contracts/fixtures/client-protocol/`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use agora_protocol::{
    ClientMessage, ErrorCode, PROTOCOL_VERSION, ProtocolVersion, RequestId, ServerMessage, StateId,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn fixture_dir(kind: &str, side: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/fixtures/client-protocol")
        .join(kind)
        .join(side)
}

fn fixtures(kind: &str, side: &str) -> Vec<(String, String)> {
    let mut files: Vec<_> = fs::read_dir(fixture_dir(kind, side))
        .expect("fixture directory exists")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(&path).unwrap())
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no {kind}/{side} fixtures");
    files
}

/// Parse each valid fixture, re-serialize it, and require the same JSON value. Returns the
/// message types covered.
fn round_trip_valid<T: DeserializeOwned + Serialize>(side: &str) -> BTreeSet<String> {
    let mut types = BTreeSet::new();
    for (name, text) in fixtures("valid", side) {
        let original: Value = serde_json::from_str(&text).unwrap();
        let message: T = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{side}/{name} failed to parse: {e}"));
        let reserialized = serde_json::to_value(&message).unwrap();
        assert_eq!(reserialized, original, "{side}/{name} did not round-trip");
        types.insert(original["type"].as_str().unwrap().to_owned());
    }
    types
}

fn reject_invalid<T: DeserializeOwned + std::fmt::Debug>(side: &str) {
    for (name, text) in fixtures("invalid", side) {
        let result = serde_json::from_str::<T>(&text);
        assert!(
            result.is_err(),
            "{side}/{name} should be rejected: {result:?}"
        );
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn r01_r08_valid_client_fixtures_round_trip_and_cover_every_type() {
    let covered = round_trip_valid::<ClientMessage>("client");
    assert_eq!(covered, set(ClientMessage::TYPES));
}

#[test]
fn r01_r08_valid_server_fixtures_round_trip_and_cover_every_type() {
    let covered = round_trip_valid::<ServerMessage>("server");
    assert_eq!(
        covered,
        set(&[
            "welcome",
            "run_created",
            "run_joined",
            "spawned",
            "started",
            "error"
        ])
    );
}

#[test]
fn r03_r08_invalid_fixtures_are_rejected() {
    reject_invalid::<ClientMessage>("client");
    reject_invalid::<ServerMessage>("server");
}

#[test]
fn r02_unknown_fields_are_ignored() {
    let text = r#"{ "type": "start", "request_id": 4, "added_in_a_later_minor": true }"#;
    let message: ClientMessage = serde_json::from_str(text).unwrap();
    assert_eq!(
        message,
        ClientMessage::Start {
            request_id: RequestId::new(4).unwrap()
        }
    );
}

#[test]
fn r03_integer_bounds() {
    assert!(RequestId::new(0).is_none());
    assert!(RequestId::new(1).is_some());
    assert!(RequestId::new((1 << 53) - 1).is_some());
    assert!(RequestId::new(1 << 53).is_none());
    assert_eq!(StateId::new(0).map(StateId::get), Some(0));
}

#[test]
fn r05_versions_parse_as_semver_core() {
    assert_eq!("0.1.0".parse(), Ok(ProtocolVersion::new(0, 1, 0)));
    assert_eq!("10.20.30".parse(), Ok(ProtocolVersion::new(10, 20, 30)));
    for bad in [
        "",
        "1",
        "1.0",
        "1.0.0.0",
        "01.0.0",
        "1.+0.0",
        "1.0.0-alpha",
        " 1.0.0",
        "a.b.c",
    ] {
        assert!(bad.parse::<ProtocolVersion>().is_err(), "{bad:?}");
    }
    assert_eq!(PROTOCOL_VERSION.to_string(), "0.1.0");
}

#[test]
fn r06_mvp_compatibility_requires_an_exact_match() {
    let v = PROTOCOL_VERSION;
    assert!(v.is_compatible_with(v));
    for other in [
        ProtocolVersion::new(0, 1, 1),
        ProtocolVersion::new(0, 2, 0),
        ProtocolVersion::new(1, 1, 0),
    ] {
        assert!(!v.is_compatible_with(other), "{other}");
    }
}

#[test]
fn r07_only_hello_lacks_a_request_id() {
    for (name, text) in fixtures("valid", "client") {
        let message: ClientMessage = serde_json::from_str(&text).unwrap();
        let is_hello = matches!(message, ClientMessage::Hello { .. });
        assert_eq!(message.request_id().is_none(), is_hello, "{name}");
    }
}

#[test]
fn r09_unrecognized_error_codes_are_preserved() {
    let text = r#"{ "type": "error", "request_id": 7, "code": "added_later", "message": "m" }"#;
    let ServerMessage::Error(error) = serde_json::from_str(text).unwrap() else {
        panic!("expected an error message");
    };
    assert_eq!(error.code, ErrorCode::Unrecognized("added_later".into()));
    let round_trip: Value = serde_json::to_value(ServerMessage::Error(error)).unwrap();
    assert_eq!(round_trip["code"], "added_later");
}

#[test]
fn r09_known_error_codes_use_their_wire_names() {
    let text =
        r#"{ "type": "error", "request_id": null, "code": "malformed_message", "message": "m" }"#;
    let ServerMessage::Error(error) = serde_json::from_str(text).unwrap() else {
        panic!("expected an error message");
    };
    assert_eq!(error.code, ErrorCode::MalformedMessage);
    assert_eq!(error.request_id, None);
}
