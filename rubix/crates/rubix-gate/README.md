# rubix-gate

The access & policy gate for the rubix platform — the read and write enforcement point.

## What it provides

- **Read path** — `authenticate` a principal, `issue_scoped_session` to mint a scoped SurrealDB session, then `read_record_on_session` / `read_records_on_session(_filtered)`. Reads are enforced by SurrealDB row-level permissions, never proxied per message.
- **Capability layer** (`capability`) — app-enforced authz over cross-plane actions: `Grant`, `create_grant`, `revoke_grant`, `list_grants`, `check_capability`. Fails closed.
- **Write path** (`command`) — every mutation crosses the gate as a `Command`: it checks the grant, captures before/after atomically, mints/carries the correlation id, applies the change, and writes an immutable audit row.
- **Audit** (`AuditRecord`, `define_audit_schema`) — the append-only, immutable audit log.
- **Undo/redo** — change classification and the reversible-change stack.

## Where it sits

The chokepoint between callers and the store for anything sensitive. Users and extensions share one identity model (`rubix_core::Principal`) and both cross this gate identically.

Authority: `rubix/STACK-DEISGN.md` contracts #1, #2, #3, #4; `rubix/docs/SCOPE.md` principles 5 and 7.
