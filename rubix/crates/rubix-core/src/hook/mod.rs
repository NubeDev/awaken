//! Hooks — fire a rule on a record write, declared as data.
//!
//! PocketBase hooks run on record events; rubix expresses the same as a
//! `kind: "hook"` record (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "Server-side hooks"), so binding a side-effect to a write is data and crosses
//! the gate like any other record. This module owns the domain model — the
//! [`HookEvent`] vocabulary, the parsed [`Hook`] binding and its match predicate,
//! and the loader that reads a namespace's hooks. *Triggering* a matched hook
//! (invoking the bound rule through the gate, off the row-perm-scoped live-query
//! plane) lives in `rubix-rules`, which owns the rule engine; this crate sits
//! below it and only models the binding.
//!
//! These are **after-hooks**: they react to a committed write. Before-hooks that
//! can *reject* a write would have to run inside the gate's `apply()` (open
//! question 9) and are intentionally out of scope here.

mod def;
mod event;
mod load;

pub use def::{HOOK_KIND, Hook, HookParseError};
pub use event::HookEvent;
pub use load::find_hooks;
