//! The `agora-viewer` executable.

use agora_client::protocol::RunId;
use agora_viewer::network::{self, Target};
use bevy::app::AppExit;
use clap::{ArgGroup, Parser};

/// Watch an Agora run: draw its grid and agents, start it, and control its pacing.
#[derive(Parser)]
#[command(version, group(ArgGroup::new("run").required(true).args(["create", "join"])))]
struct Args {
    /// Server WebSocket URL.
    #[arg(long, default_value = "ws://127.0.0.1:7878")]
    server: String,
    /// Create a run on the bundled 10 × 10 grid and watch it.
    #[arg(long)]
    create: bool,
    /// Watch an existing run.
    #[arg(long, value_name = "RUN_ID", value_parser = parse_run_id)]
    join: Option<RunId>,
}

fn parse_run_id(text: &str) -> Result<RunId, String> {
    RunId::new(text).ok_or_else(|| "the run ID is empty".to_owned())
}

fn main() -> AppExit {
    let args = Args::parse();
    let target = match args.join {
        Some(run_id) => Target::Join(run_id),
        None => Target::Create,
    };
    let title = match &target {
        Target::Create => "Agora viewer".to_owned(),
        Target::Join(run_id) => format!("Agora viewer: {run_id}"),
    };
    agora_viewer::run(network::start(args.server, target), title)
}
