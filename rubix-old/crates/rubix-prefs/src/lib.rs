//! User & org preferences and units conversion for rubix.
//!
//! Vendored from the starter platform's `starter-spi` (units +
//! preferences wire types) and `starter-prefs` (the pure three-layer
//! resolver). Those modules are self-contained — closed `Quantity` /
//! `Unit` enums, `uom`-backed conversion math, the resolved-preferences
//! DTO, and the `user → org → default` resolver — with no dependency on
//! the rest of starter's workspace, so they live here as the stable
//! core rubix builds its prefs store + HTTP edge on top of.
//!
//! What is **not** here: persistence and HTTP. The storage layer is
//! rubix-native (synchronous `rusqlite` / `postgres` via the server's
//! `Store`), and the routes + `Accept-Units` middleware are wired in
//! `rubix-server` against rubix's own `Principal`. This crate is pure:
//! types + conversion + resolution, no I/O.
//!
//! See `WS-11_UNITS_AND_PREFS.md` for the design and the
//! "convert at the presentation edge, never in storage" rule.

pub mod preferences;
pub mod resolver;
mod series;
pub mod units;

pub use series::{FromCanonicalSeries, SeriesEnvelope, SeriesPoint, ToCanonicalSeries};
