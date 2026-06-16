//! Resolve a rule's declared input bindings to DataFusion window values.
//!
//! A rule decides on time-window rollups of a numeric series; DataFusion owns
//! that data, Rhai owns the decision (`rubix/STACK-DEISGN.md`, "Rhai owns the
//! decision; DataFusion owns the data"). A [`Binding`] declares *which* window
//! value a script input takes: the canonical table, the numeric `content.<field>`
//! series, the bucket [`Grain`], which [`Aggregate`] of the latest bucket to read,
//! and the script variable name to bind it to. Resolving a binding pulls the
//! rollup through the principal's scoped session (so SurrealDB row-level
//! permissions decide the rows, contract #1) and selects the value from the most
//! recent bucket — the current window the rule fires on.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::{BucketRollup, CanonicalTable, Grain, SeriesFilter, rollup_window_filtered};

use crate::error::{Result, RuleError};

/// Which aggregate of a window bucket a binding reads.
///
/// The rollup computes every aggregate per bucket
/// (`avg/min/max/sum/count/first/last`, `rubix-query`); a binding selects the one
/// the rule decides on. Keeping selection explicit means the same series can feed
/// two inputs (e.g. `avg` and `max`) without recomputing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregate {
    /// The bucket mean.
    Avg,
    /// The bucket minimum.
    Min,
    /// The bucket maximum.
    Max,
    /// The bucket sum.
    Sum,
    /// The number of samples in the bucket.
    Count,
    /// The earliest sample (by timestamp) in the bucket.
    First,
    /// The latest sample (by timestamp) in the bucket.
    Last,
}

impl Aggregate {
    /// Select this aggregate's value from a rolled-up bucket.
    #[must_use]
    pub fn select(self, bucket: &BucketRollup) -> f64 {
        match self {
            Aggregate::Avg => bucket.avg,
            Aggregate::Min => bucket.min,
            Aggregate::Max => bucket.max,
            Aggregate::Sum => bucket.sum,
            #[allow(clippy::cast_precision_loss)]
            Aggregate::Count => bucket.count as f64,
            Aggregate::First => bucket.first,
            Aggregate::Last => bucket.last,
        }
    }
}

/// One declared input a rule's script reads as a window value.
///
/// The script sees the value under [`Binding::name`]; the rest declares how it is
/// computed from a canonical table's numeric series.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The script variable name this value is bound to.
    pub name: String,
    /// The canonical table the series is read from.
    pub table: CanonicalTable,
    /// The numeric `content.<field>` series rolled up.
    pub field: String,
    /// The bucket width the series is rolled up at.
    pub grain: Grain,
    /// Which bucket aggregate the rule decides on.
    pub aggregate: Aggregate,
    /// An optional `(content key, value)` equality narrowing the series.
    ///
    /// Many rows can share the numeric `field` across categories — every reading
    /// stores its number at `content.value` regardless of `content.measure`. A
    /// filter of `("measure", "temp")` restricts the rollup to one category so the
    /// rule decides on just that metric rather than a blend. `None` reads the
    /// whole series.
    pub filter: Option<(String, String)>,
}

impl Binding {
    /// Declare a binding of `name` to the `aggregate` of `field` at `grain` over
    /// `table`, reading the whole series (no filter).
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        table: CanonicalTable,
        field: impl Into<String>,
        grain: Grain,
        aggregate: Aggregate,
    ) -> Self {
        Self {
            name: name.into(),
            table,
            field: field.into(),
            grain,
            aggregate,
            filter: None,
        }
    }

    /// Narrow this binding's series to rows whose `content.<key>` equals `value`.
    ///
    /// The composable way to target one category of a shared numeric field — e.g.
    /// `Binding::new(…, "value", …).filtered_by("measure", "temp")` rolls up only
    /// the temperature readings. Returns the updated binding.
    #[must_use]
    pub fn filtered_by(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filter = Some((key.into(), value.into()));
        self
    }
}

/// Resolve `binding` to the window value the script will read, scoped to the
/// principal of `session`.
///
/// Pulls the rollup through the scoped session and selects the binding's
/// aggregate from the **most recent** bucket — the current window the rule fires
/// on. A series with no buckets (no readings in range, or none numeric) cannot
/// produce a value: that is a binding failure, not a silent zero, so a rule never
/// fires on a value that was never observed (no fallback that hides missing data,
/// CLAUDE.md "Core Rules").
///
/// # Errors
/// Returns [`RuleError::Window`] if the rollup scan fails, or
/// [`RuleError::Binding`] if the series yielded no bucket to read.
pub async fn resolve(session: &Surreal<Db>, binding: &Binding) -> Result<f64> {
    let filter = binding
        .filter
        .as_ref()
        .map(|(key, value)| SeriesFilter { key, value });
    let buckets = rollup_window_filtered(session, binding.table, &binding.field, binding.grain, filter)
        .await
        .map_err(|e| RuleError::Window(e.to_string()))?;
    let latest = buckets.last().ok_or_else(|| {
        RuleError::Binding(format!(
            "no window bucket for '{}' from content.{}",
            binding.name, binding.field
        ))
    })?;
    Ok(binding.aggregate.select(latest))
}

#[cfg(test)]
mod tests {
    use rubix_query::{BucketRollup, CanonicalTable, Grain};

    use super::{Aggregate, Binding};

    fn bucket() -> BucketRollup {
        BucketRollup {
            bucket_start: 0,
            avg: 20.0,
            min: 10.0,
            max: 30.0,
            sum: 60.0,
            count: 3,
            first: 10.0,
            last: 30.0,
        }
    }

    #[test]
    fn each_aggregate_selects_its_field() {
        let b = bucket();
        assert_eq!(Aggregate::Avg.select(&b), 20.0);
        assert_eq!(Aggregate::Min.select(&b), 10.0);
        assert_eq!(Aggregate::Max.select(&b), 30.0);
        assert_eq!(Aggregate::Sum.select(&b), 60.0);
        assert_eq!(Aggregate::Count.select(&b), 3.0);
        assert_eq!(Aggregate::First.select(&b), 10.0);
        assert_eq!(Aggregate::Last.select(&b), 30.0);
    }

    #[test]
    fn binding_carries_its_declaration() {
        let binding = Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        );
        assert_eq!(binding.name, "temp");
        assert_eq!(binding.field, "temperature");
        assert_eq!(binding.grain, Grain::Minute);
        assert_eq!(binding.aggregate, Aggregate::Avg);
    }
}
