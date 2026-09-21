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

- [Simulation — CDD-001](simulation/cdd.md) (draft): step lifecycle, world state, actions, and perception boundaries; proposed package `agora-sim`.
