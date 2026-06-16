//! Server-side transform stage — the aggregate tier of the hybrid spec (§1).
//!
//! Transforms are a portable, declarative post-query pipeline stored on the chart
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §1). Execution is split by class: the
//! aggregate ops (`filter`/`groupBy`/`reduce`) shrink the wire and need the full
//! dataset, so they run here as a small DataFusion stage over the result batches;
//! the cosmetic ops (`rename`/`calculated`/`organize`) run client-side via nexus's
//! executor and are no-ops here. The backend receives the **whole** spec (the
//! contract stays portable) and applies only the aggregate subset.

mod execute;
mod spec;

use datafusion::arrow::record_batch::RecordBatch;

use crate::error::Result;

pub use execute::apply_aggregate_transforms;
pub use spec::{Agg, CompareOp, ReduceCalc, Transform};

/// Apply the server-side (aggregate) tier of `transforms` to `batches`.
///
/// A thin alias over [`apply_aggregate_transforms`] kept at the module root so the
/// transport layer calls one well-named seam; cosmetic transforms pass through
/// untouched for the client to run.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if a transform names an invalid
/// identifier or a generated statement fails.
pub async fn apply_transforms(
    batches: Vec<RecordBatch>,
    transforms: &[Transform],
) -> Result<Vec<RecordBatch>> {
    apply_aggregate_transforms(batches, transforms).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datafusion::arrow::array::{Float64Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;

    use super::spec::{Agg, CompareOp, ReduceCalc, Transform};
    use super::*;

    // A small table: city (utf8) + temp (f64), three rows.
    fn sample() -> Vec<RecordBatch> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("city", DataType::Utf8, false),
            Field::new("temp", DataType::Float64, false),
        ]));
        let cities = StringArray::from(vec!["oslo", "oslo", "cairo"]);
        let temps = Float64Array::from(vec![1.0, 3.0, 40.0]);
        vec![RecordBatch::try_new(schema, vec![Arc::new(cities), Arc::new(temps)]).unwrap()]
    }

    fn rows(batches: &[RecordBatch]) -> usize {
        batches.iter().map(RecordBatch::num_rows).sum()
    }

    #[tokio::test]
    async fn no_aggregate_transform_passes_through() {
        let input = sample();
        let out = apply_transforms(
            input.clone(),
            &[Transform::Rename {
                from: "city".into(),
                to: "place".into(),
            }],
        )
        .await
        .unwrap();
        assert_eq!(rows(&out), 3, "cosmetic-only spec leaves rows unchanged");
    }

    #[tokio::test]
    async fn filter_drops_rows() {
        let out = apply_transforms(
            sample(),
            &[Transform::Filter {
                field: "temp".into(),
                op: CompareOp::Gt,
                value: "10".into(),
            }],
        )
        .await
        .unwrap();
        assert_eq!(rows(&out), 1, "only cairo (40) exceeds 10");
    }

    #[tokio::test]
    async fn group_by_collapses_to_one_row_per_key() {
        let out = apply_transforms(
            sample(),
            &[Transform::GroupBy {
                by: "city".into(),
                field: "temp".into(),
                agg: Agg::Avg,
                as_: "avg_temp".into(),
            }],
        )
        .await
        .unwrap();
        assert_eq!(rows(&out), 2, "oslo + cairo");
        let schema = out[0].schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(names, ["city", "avg_temp"]);
    }

    #[tokio::test]
    async fn reduce_collapses_to_a_single_row() {
        let out = apply_transforms(
            sample(),
            &[Transform::Reduce {
                field: "temp".into(),
                calc: ReduceCalc::Sum,
                as_: "total".into(),
            }],
        )
        .await
        .unwrap();
        assert_eq!(rows(&out), 1);
    }

    #[tokio::test]
    async fn aggregate_steps_chain_in_order() {
        // Filter to oslo's two rows, then sum → one row of 4.0.
        let out = apply_transforms(
            sample(),
            &[
                Transform::Filter {
                    field: "city".into(),
                    op: CompareOp::Eq,
                    value: "oslo".into(),
                },
                Transform::Reduce {
                    field: "temp".into(),
                    calc: ReduceCalc::Sum,
                    as_: "total".into(),
                },
            ],
        )
        .await
        .unwrap();
        assert_eq!(rows(&out), 1);
        let col = out[0]
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(col.value(0), 4.0);
    }

    #[tokio::test]
    async fn an_invalid_identifier_is_rejected_not_injected() {
        let err = apply_transforms(
            sample(),
            &[Transform::Filter {
                field: "temp\"; DROP".into(),
                op: CompareOp::Gt,
                value: "0".into(),
            }],
        )
        .await
        .unwrap_err();
        assert!(matches!(err, crate::QueryError::Rejected(_)), "got {err:?}");
    }
}
