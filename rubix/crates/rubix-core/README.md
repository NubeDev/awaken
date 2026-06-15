# rubix-core

Domain model and shared contracts for the rubix platform — the types every other crate builds on.

## What it provides

- **`Principal` / `PrincipalKind` / `Role`** — the single identity model shared by users and extensions.
- **`Record` + CRUD verbs** (`create_record`, `read_record`, `update_record`, `delete_record`, `list_records`, `list_records_filtered`) — the canonical record.
- **`CollectionDef` / `FieldDef`** — collection (schema) definitions, validation, and meta-collection bootstrap.
- **`Tag`** — tagging and `find_records_by_tags`.
- **`Id`, `CorrelationId`** — id and correlation primitives.
- **`Profile`, `RuntimeConfig`, `StoreEngine`** — runtime configuration.
- **`Error` / `Result` / `ResultExt`** — the project error type and `.context()` chaining.

## Where it sits

Bottom of the stack. Has no awareness of transport, storage engine, or the gate — it defines the shapes those layers move around.

Authority: `rubix/docs/SCOPE.md`; contracts #3 and #6 in `rubix/STACK-DEISGN.md`.
