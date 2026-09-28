//! `agora-demo`: a client whose agents move at random.
//!
//! `agora-demo create` creates a run, prints its ID, and starts it once the run has enough
//! agents. `agora-demo join <RUN_ID>` joins a run. Both spawn agents and move each one a random
//! cardinal step every state until stopped.

use std::process::ExitCode;

use agora_client::protocol::{
    Action, ActionEntry, CatalogEntryId, Direction, PacingMode, Placement, RunId,
};
use agora_client::{Client, ClientError, Observations, Session};
use clap::{Parser, Subcommand};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// The catalog entry `create` uses.
const CATALOG_ENTRY: &str = "empty-grid-10x10";

/// A demo Agora client whose agents move at random.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Server WebSocket URL.
    #[arg(long, default_value = "ws://127.0.0.1:7878")]
    server: String,
    /// Agents to spawn.
    #[arg(long, default_value_t = 1)]
    agents: u32,
    /// Seed for the random moves. Omit it for a random seed.
    #[arg(long)]
    seed: Option<u64>,
    /// Stop after this many steps. Omit it to run until interrupted.
    #[arg(long)]
    steps: Option<u64>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a run, print its ID, and start it once it has enough agents.
    Create {
        /// Start once the run has this many agents in total, counting other clients'.
        /// Defaults to --agents.
        #[arg(long)]
        start_at: Option<usize>,
        /// Take pacing control and run as fast as the agents submit. Creating a run watches
        /// it, which otherwise paces steps at the server's default interval.
        #[arg(long)]
        unlimited: bool,
    },
    /// Join an existing run.
    Join {
        /// The run ID printed by `create`.
        #[arg(value_parser = parse_run_id)]
        run_id: RunId,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    tokio::select! {
        result = run(args) => match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                error!("{e}");
                ExitCode::FAILURE
            }
        },
        _ = tokio::signal::ctrl_c() => {
            info!("interrupted");
            ExitCode::SUCCESS
        }
    }
}

async fn run(args: Args) -> Result<(), ClientError> {
    let client = Client::connect(&args.server).await?;
    let (session, mut observations) = match &args.command {
        Command::Create { .. } => {
            let entry = CatalogEntryId::new(CATALOG_ENTRY).expect("the entry ID is non-empty");
            let established = client.create_run(entry).await?;
            // Printed on stdout so scripts and other terminals can pick it up.
            println!("{}", established.0.info().run_id);
            established
        }
        Command::Join { run_id } => client.join_run(run_id.clone()).await?,
    };
    info!(run = %session.info().run_id, "session established");

    for _ in 0..args.agents {
        let agent = session.spawn(Placement::Random).await?;
        info!(agent = agent.get(), "agent spawned");
    }

    if let Command::Create {
        start_at,
        unlimited,
    } = args.command
    {
        start_when_ready(
            &session,
            start_at.unwrap_or(args.agents as usize),
            unlimited,
        )
        .await?;
    }

    let mut rng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => rand::make_rng(),
    };
    move_randomly(&session, &mut observations, &mut rng, args.steps).await
}

fn parse_run_id(text: &str) -> Result<RunId, String> {
    RunId::new(text).ok_or_else(|| "the run ID is empty".to_owned())
}

/// Watch the run until it has `agents` agents, then start it.
async fn start_when_ready(
    session: &Session,
    agents: usize,
    unlimited: bool,
) -> Result<(), ClientError> {
    session.watch().await?;
    if unlimited {
        session.claim_pacing().await?;
        session.set_pacing(PacingMode::Unlimited).await?;
    }
    let mut view = session.view();
    info!(agents, "waiting for agents");
    loop {
        let count = view
            .borrow_and_update()
            .as_ref()
            .map_or(0, |v| v.agents.len());
        if count >= agents {
            break;
        }
        view.changed().await.map_err(|_| ClientError::Closed)?;
    }
    session.start().await?;
    info!("run started");
    Ok(())
}

/// Answer each observation batch with a random step for every agent in it.
async fn move_randomly(
    session: &Session,
    observations: &mut Observations,
    rng: &mut StdRng,
    steps: Option<u64>,
) -> Result<(), ClientError> {
    const DIRECTIONS: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    while let Some(batch) = observations.next().await {
        let state = batch.state_id.get();
        info!(state, "observed");
        if steps.is_some_and(|steps| state >= steps) {
            // Leave cleanly, so the run does not wait for this session to expire.
            return session.leave().await;
        }
        let actions = batch
            .observations
            .iter()
            .map(|observation| ActionEntry {
                agent_id: observation.agent_id,
                action: Action::Move {
                    direction: DIRECTIONS[rng.random_range(0..DIRECTIONS.len())],
                    distance: 1,
                },
            })
            .collect();
        for result in session.submit(batch.state_id, actions).await? {
            if let Some(error) = result.error {
                warn!(agent = result.agent_id.get(), code = %error.code, "action rejected");
            }
        }
    }
    Err(session
        .closed_reason()
        .map_or(ClientError::Closed, ClientError::RunClosed))
}
