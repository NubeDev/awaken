# Rubix Fleet/Dashboard Build — Blockers & Follow-ups

Append-only log of things an unattended session could NOT do properly and refused to hack. The human
resolves an entry, then strikes it through (`~~...~~`) or deletes it; the loop resets the
corresponding ⛔ row to ⬜ on its next wake.

## Format

```
### <utc-date> — <WS-xx> — <one-line title>
- **What's blocked:** ...
- **Why (the ambiguity / missing dep / guardrail conflict):** ...
- **What the human must decide/provide:** ...
- **Committed so far:** <commit sha or "nothing — clean working tree">
```

---

### 2026-06-13 — WS-02 — `datasource`-kind variable options need a datasources list endpoint
- **What's blocked:** A `datasource`-kind variable should offer the org's available data sources as its option list. The UI has no endpoint that enumerates datasources, so its options resolve empty (`use-resolution.ts` `optionsFor` returns `[]` for this kind).
- **Why (the ambiguity / missing dep / guardrail conflict):** This is a missing dependency, not a hack — every other variable kind resolves fully. Inventing a fake endpoint or hardcoding sources would violate "no placeholder implementations / avoid fallbacks". The variable model, editor, and resolution pipeline all already accept the kind; only the option source is absent.
- **What the human must decide/provide:** Whether to expose a `GET /datasources` (org-scoped) list and what shape it returns; then wire it into `useVariableResolution` alongside `useSites`.
- **Committed so far:** the full WS-02 feature compiles and is green; only `datasource` options are empty pending the endpoint.

---

### 2026-06-13 — WS-05 — pre-existing zenoh-mesh flake in `rubix-driver-sim` supervised test
- **What's blocked:** Nothing in WS-05. Surfaced incidentally during the `cargo test --workspace` gate.
- **Why (the ambiguity / missing dep / guardrail conflict):** `rubix-driver-sim::supervised::out_of_grant_publish_is_refused_locally` fails on a 3-second `recv_async` timing window over a real zenoh mesh. It is timing-sensitive and touches no WS-05 code (WS-05 owns nothing in `rubix-driver-sim` or the bus). Not a regression introduced here.
- **What the human must decide/provide:** Whether to stabilise the driver-sim grant-enforcement timing assertion (e.g. widen the window or make it event-driven). Out of scope for WS-05.
- **Committed so far:** WS-05 API section `3d20b583`; core `09541e1d`; store `5beda3e7`. `rubix-server` fully green (172 api + 90 lib + 6 migrate), clippy clean default + cloud.
