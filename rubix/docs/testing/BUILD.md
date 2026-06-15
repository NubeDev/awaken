# BUILD — Cargo test + clippy verification

> Verified: main (2026-06-15)
> This is the **gate every WS must pass** before landing on the branch.

---

## The gates

Every workstream ships with:

1. **Unit + integration tests** that run locally and in CI
2. **Clippy warnings** eliminated (no `-W` allowed)
3. **All tests green** across the whole workspace
4. **No temporary stubs** (`todo!()`, `unimplemented!()`, `#[ignore]`)

---

## Run the full suite

From `rubix/`:

```bash
# Run all tests
cargo test --workspace

# Run all checks (code quality)
cargo clippy --workspace --all-targets

# Or use the Makefile
make test
make lint
```

### Expected output

```
$ cargo test --workspace

...
   Compiling rubix-core v0.1.0
   Compiling rubix-store v0.1.0
   Compiling rubix-gate v0.1.0
   Compiling rubix-bus v0.1.0
   Compiling rubix-query v0.1.0
   Compiling rubix-rules v0.1.0
   Compiling rubix-ingest v0.1.0
   Compiling rubix-server v0.1.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in XXs

running X tests
...
test result: ok. X passed; 0 failed; 0 ignored; 0 measured

$ cargo clippy --workspace --all-targets

...
   Checking rubix-core v0.1.0
   Checking rubix-store v0.1.0
   ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in XXs
```

No warnings. No failures. Clean exit.

---

## Test breakdown by crate

| Crate | Purpose | Test count | Type |
|-------|---------|-----------|------|
| `rubix-core` | IDs, error enum, config | ~20 | unit |
| `rubix-store` | SurrealDB boundary + schema | ~30 | unit + integration |
| `rubix-gate` | Principal, capability, command | ~40 | unit + integration |
| `rubix-bus` | Control + live-query events | ~15 | unit + integration |
| `rubix-query` | DataFusion surface | ~20 | unit + integration |
| `rubix-rules` | Rhai sandbox + runtime | ~30 | unit + integration |
| `rubix-ingest` | Zenoh pre-processing | ~20 | unit + integration |
| `rubix-server` | HTTP routes + integration | ~25 | integration |

Total: ~180 tests across the backend.

---

## Common failures and fixes

### ❌ "cannot find rubix_store" / build error

**Cause:** Dependency not committed yet.  
**Fix:** Check `cargo.toml` ordering; run `cargo build` to diagnose.

### ❌ Test panics with "database unavailable"

**Cause:** SurrealDB connection failed (embedded kv-mem mode should not fail).  
**Fix:** Check `RUBIX_DB` env var; if set to a file path, ensure the file is writable.  
Clear with: `rm rubix.db` if using file-based store.

### ❌ Clippy warning: "this could be a const fn"

**Cause:** Code can be simplified.  
**Fix:** Apply the suggestion or refactor as the warning directs. No `-A` suppression.

### ❌ Tests timeout (take > 30s)

**Cause:** Long-running integration test or deadlock.  
**Fix:** Increase timeout: `cargo test -- --test-threads=1 --nocapture`.

---

## Running tests in isolation

Test a single crate:

```bash
cargo test -p rubix-gate
```

Test a single test function:

```bash
cargo test -p rubix-gate -- test_principal_create
```

Run with output visible (useful for debugging):

```bash
cargo test --workspace -- --nocapture
```

Run tests sequentially (avoids race conditions):

```bash
cargo test --workspace -- --test-threads=1
```

---

## Integration tests (tests/ directory)

Each crate may have an integration test suite under `crates/crate-name/tests/`.

Run them:

```bash
cargo test --test '*'
```

Integration tests can:
- Spawn a full SurrealDB instance (in-memory kv-mem)
- Call public APIs across crate boundaries
- Verify end-to-end workflows (e.g., write + audit + undo)

---

## Pre-commit checklist

Before committing a WS:

```bash
# 1. Run tests
cargo test --workspace

# 2. Check code quality
cargo clippy --workspace --all-targets

# 3. Format
cargo fmt

# 4. Verify git state
git status  # should be clean except your changes
git diff    # review your changes
```

All green? Commit with the WS-xx prefix:

```bash
git commit -m "WS-10: ✨ feat(rubix-datasource): pluggable connector framework"
```

---

## CI integration (planned)

Once the project is on CI/CD (GitHub Actions), these gates will run automatically:

- `cargo test --workspace` — all tests must pass
- `cargo clippy --workspace --all-targets` — no warnings
- `cargo fmt --check` — code must be formatted
- File size check — no Rust file > 400 lines

Locally, enforce with:

```bash
./scripts/check-file-size.sh    # 400-line guard
cargo fmt --check
```

---

## Next steps

- **Tests passing?** Pick a feature doc and run the runbook.
- **Test failing?** Go to `feedback-loop/CAPTURE.md` and produce evidence.
- **Adding a new test?** Follow the crate's pattern; mirror `src/` in `tests/`.
