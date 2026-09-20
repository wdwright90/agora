# Context and evidence

- [SADD](../../docs/architecture/sadd.md): current project direction and first milestone.
- [Rust workspace](../../rust/README.md): empty scaffold; package boundaries are undecided.
- [Migration inventory](../../docs/migration/inventory.md): disposition tracking for historical code.

## POC review

Source: sibling `agora-poc`, revision `f0b9fd26775251a318e906da13d1cecf81f2431e`. At review, Git reported only an untracked `.claude/` directory; it was not inspected. No tracked modifications were reported.

Read `README.md`, `Cargo.toml`, `crates/agora-sim/src/lib.rs`, `crates/agora-server/src/lib.rs`, `crates/agora-viewer/src/main.rs`, and `crates/agora-trainer/Cargo.toml`; enumerated Rust source paths. This is a targeted structural review, not a full implementation audit.

- The POC separates protocol, simulation, server, viewer, reference client, and Rust trainer into packages. The README omits the trainer listed in the source tree.
- The simulation exposes an `Environment` trait and contains built-in environments, recording, snapshots, registry, and tick-loop modules.
- The server creates one session/environment per connection. This differs from Agora's requirement for different clients to share a simulation.
- The viewer reads recorded snapshots or follows a growing recording file. Its entry point describes live socket viewing as future work. Agora requires observers to connect to ongoing simulations and interact with their state.
- The POC includes a Rust PPO trainer. Current Agora direction puts example training and analysis tooling in Python; this package is not automatically part of the successor.

These findings are historical evidence. No package split, dependency, transport, or code reuse has been adopted. No POC builds or tests were run.
