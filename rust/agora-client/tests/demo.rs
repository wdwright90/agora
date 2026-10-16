//! Acceptance tests for the `agora-demo` executable (SPEC-004-R08): a `create` process and a
//! `join` process, each with one agent, run a shared run for a few steps and exit, and a client
//! without a step limit leaves once its agents have starved.

use std::process::Stdio;
use std::time::Duration;

use agora_server::{ServerConfig, serve};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;

const TIMEOUT: Duration = Duration::from_secs(30);

fn demo(url: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agora-demo"));
    command
        .args(["--server", url, "--seed", "7", "--steps", "3"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}

#[tokio::test]
async fn r08_two_demo_clients_share_a_run_until_their_step_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(serve(listener, ServerConfig::default()));

    let mut creator = demo(&url)
        .args(["create", "--start-at", "2", "--unlimited"])
        .spawn()
        .unwrap();
    let stdout = creator.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let run_id = tokio::time::timeout(TIMEOUT, lines.next_line())
        .await
        .expect("timed out waiting for the run ID")
        .unwrap()
        .expect("create printed nothing");

    let joiner = demo(&url).args(["join", &run_id]).spawn().unwrap();

    let (created, joined) = tokio::time::timeout(TIMEOUT, async {
        tokio::join!(creator.wait(), joiner.wait_with_output())
    })
    .await
    .expect("the demo clients did not finish");
    assert!(created.unwrap().success());
    assert!(joined.unwrap().status.success());
}

#[tokio::test]
async fn r08_a_demo_client_leaves_once_its_foragers_starve() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(serve(listener, ServerConfig::default()));

    // No step limit: the client runs until both of its foragers have starved.
    let mut command = Command::new(env!("CARGO_BIN_EXE_agora-demo"));
    command
        .args(["--server", &url, "--seed", "7", "--agents", "2"])
        .args(["create", "--unlimited"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let status = tokio::time::timeout(TIMEOUT, command.status())
        .await
        .expect("the demo client did not finish")
        .unwrap();
    assert!(status.success());
}
