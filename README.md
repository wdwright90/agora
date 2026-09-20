# Agora

Agora is a simulation framework for training, testing, and observing AI agents interacting with different simulations and with each other. Built in Rust using the Bevy game engine, it will provide an API for clients to submit agent actions and receive observations. Clients are responsible for updating their model weights and may use any language or client type that fulfills the framework's message contracts.

The initial goal is a 2D, top-down simulation supporting multiple agent types connected from different clients. The framework will provide tools to create, organize, and store user-defined or randomly generated environments, with building components such as walls, holes, food tokens, doors, levers, and buttons.

Visualization will be optional. Users will be able to connect to an ongoing simulation through a Bevy visualization, pause or run it, record and play back runs, and modify its state during observation to see how agents react. The project will include an example Python client with tooling for training different model architectures and understanding their behavior.

See the SADD's [purpose and scope](docs/architecture/sadd.md#purpose-and-scope) for the project definition. Detailed behavior and component boundaries remain to be defined.

The [first usable milestone](docs/architecture/sadd.md#first-usable-milestone) is a viewable small empty grid with two connected agents, a simple client that spawns an agent and submits move actions, and empty observations returned by the framework.

Start with the [documentation index](docs/index.md) and [contribution workflow](CONTRIBUTING.md).

The previous implementation is preserved separately as `agora-poc`. It is reference material, not the specification for this repository. Record explicit decisions before porting its functionality.

The [Rust workspace](rust/README.md) is scaffolded under `rust/` with no member packages yet. The rest of the repository currently contains project documentation. No application build or test commands are available yet.
