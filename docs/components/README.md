# Component documentation

Create one directory per logical component:

```text
<component>/
  cdd.md
  specs/
    <behavior>.md
```

A component may span multiple packages or languages. Its CDD describes responsibilities, internal structure, interfaces, data flow, dependencies, and rationale. It links to the SADD, its specs, and shared contracts.

Use the [CDD template](../templates/cdd.md) and [spec template](../templates/spec.md). Create specs only for concrete requirements. Shared contracts belong in [contracts](../contracts/README.md) and are referenced by every participating CDD.

- [Simulation — CDD-001](simulation/cdd.md) (draft): step lifecycle, world state, actions, and perception boundaries; package `agora-sim`; specs: [SPEC-001](simulation/specs/lifecycle-and-movement.md) (draft).
- [Server — CDD-002](server/cdd.md) (draft): connections, sessions, run hosting, and lifetimes; package `agora-server`; specs: [SPEC-003](server/specs/sessions-and-runs.md) (draft).
- [Rust client — CDD-003](client/cdd.md) (draft): the client library and demo client; package `agora-client`; specs: [SPEC-004](client/specs/client-library.md) (draft).
- [Viewer — CDD-004](viewer/cdd.md) (draft): the Bevy viewer; package `agora-viewer`; specs: [SPEC-005](viewer/specs/viewer.md) (draft).
