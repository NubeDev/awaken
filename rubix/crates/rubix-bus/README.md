# rubix-bus

The event bus for the rubix platform — the eventing spine.

## What it provides

Two eventing planes behind one surface:

- **In-process** (`inprocess`) — tokio broadcast channels for component-to-component control events inside the binary. No serialization, no network. `ControlBus`, `ControlSubscription`, `publish`, `subscribe`.
- **Data-change** (`livequery`) — SurrealDB live queries as the "a record appeared/changed" pub/sub. A subscription opens on a gate-issued **scoped session**, so SurrealDB row-level permissions decide which records a principal sees — scope is set once at subscribe, not proxied per message. `DataChange`, `DataChangeKind`, `DataChangeStream`, `subscribe_table`.

## Where it sits

The internal eventing spine. The Zenoh stream/transport plane is a separate crate (`rubix-ingest` / WS-12), not this one.

Authority: `rubix/docs/SCOPE.md` ("Event bus"); contract #1 in `rubix/STACK-DEISGN.md`.
