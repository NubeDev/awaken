//! Register the [`Frame`] custom type and its curated primitives into Rhai.
//!
//! Every method returns a new `Frame` (or a summary frame), so scripts chain
//! `df.resample(...).zscore("kw").anomalies("kw", 3.0)`. None of these iterate
//! rows in script space — the data work happens in the engine. There is
//! deliberately no `Frame` method that exposes a row to the script.

use std::sync::Arc;

use rhai::{Array, Engine, EvalAltResult, Map};

use super::bridge;
use crate::error::RuleError;
use crate::frame::Frame;
use crate::run::Execution;

/// Register `Frame` and its primitive methods into `engine`.
pub(crate) fn register_frame(engine: &mut Engine, exec: Arc<Execution>) {
    engine.register_type_with_name::<Frame>("Frame");

    macro_rules! method {
        ($name:literal, |$f:ident $(, $arg:ident : $ty:ty)*| $body:expr) => {{
            let e = exec.clone();
            engine.register_fn(
                $name,
                move |$f: &mut Frame $(, $arg: $ty)*| -> Result<Frame, Box<EvalAltResult>> {
                    let compute = || -> Result<Frame, RuleError> { $body };
                    bridge(&e, compute())
                },
            );
        }};
    }

    method!("select", |f, cols: Array| f.select(&strings(cols)?));
    method!("rename", |f, from: &str, to: &str| f.rename(from, to));
    method!("filter_gt", |f, col: &str, v: f64| f.filter_gt(col, v));
    method!("filter_lt", |f, col: &str, v: f64| f.filter_lt(col, v));
    method!("filter_eq", |f, col: &str, v: f64| f.filter_eq(col, v));
    method!("rolling_mean", |f, t: &str, c: &str, w: &str| f.rolling_mean(t, c, w));
    method!("rolling_min", |f, t: &str, c: &str, w: &str| f.rolling_min(t, c, w));
    method!("rolling_max", |f, t: &str, c: &str, w: &str| f.rolling_max(t, c, w));
    method!("rolling_sum", |f, t: &str, c: &str, w: &str| f.rolling_sum(t, c, w));
    method!("zscore", |f, col: &str| f.zscore(col));
    method!("resample", |f, t: &str, every: &str, aggs: Map| {
        f.resample(t, every, &agg_pairs(aggs)?)
    });
    method!("lag", |f, t: &str, c: &str| f.lag(t, c));
    method!("diff", |f, t: &str, c: &str| f.diff(t, c));
    method!("pct_change", |f, t: &str, c: &str| f.pct_change(t, c));
    method!("fill_null", |f, strategy: &str| f.fill_null(strategy));
    method!("head", |f, n: i64| f.head(n));
    method!("tail", |f, n: i64| f.tail(n));
    method!("sort", |f, col: &str, asc: bool| f.sort(col, asc));
    method!("anomalies", |f, col: &str, z: f64| f.anomalies(col, z));
    method!("describe", |f| f.describe());

    // Read-only inspectors a script uses to write decision logic over the frame
    // *shape*, never over individual rows.
    engine.register_fn("row_count", |f: &mut Frame| f.row_count() as i64);
    register_any_true(engine, exec);
}

/// `any_true(frame, col)` — true if any row's boolean `col` is true.
///
/// The bridge between a vectorized flag column (e.g. from `anomalies`) and the
/// script's decision, without exposing rows: the reduction runs in the engine.
fn register_any_true(engine: &mut Engine, exec: Arc<Execution>) {
    let e = exec;
    engine.register_fn(
        "any_true",
        move |f: &mut Frame, col: &str| -> Result<bool, Box<EvalAltResult>> {
            bridge(&e, f.any_true(col))
        },
    );
}

/// Coerce a Rhai array into `Vec<String>`, rejecting non-string elements.
fn strings(arr: Array) -> Result<Vec<String>, RuleError> {
    arr.into_iter()
        .map(|v| {
            v.into_string()
                .map_err(|t| RuleError::Runtime(format!("expected string column name, got {t}")))
        })
        .collect()
}

/// Coerce a Rhai map of `column -> aggregate` into ordered pairs.
fn agg_pairs(map: Map) -> Result<Vec<(String, String)>, RuleError> {
    map.into_iter()
        .map(|(k, v)| {
            let func = v.into_string().map_err(|t| {
                RuleError::Runtime(format!("resample aggregate for `{k}` must be a string, got {t}"))
            })?;
            Ok((k.to_string(), func))
        })
        .collect()
}
