//! L2-normalize an embedding before it is stored.
//!
//! `rubix-query` ranks nearest neighbours with `vector::distance::euclidean`
//! ([search/nearest.rs](../../../rubix-query/src/search/nearest.rs)). This is
//! **not** a model-choice constraint (AGENT.md, "Memory schema"; open question
//! 3c): on **L2-normalized** vectors euclidean ranking is monotonic with cosine
//! similarity — the nearest-neighbour *order* is identical — so the only
//! requirement is to normalize embeddings before insert. Both the stored memory
//! vector and the recall probe go through this verb, so write and read share one
//! geometry.

use crate::error::{AgentError, Result};

/// Return `embedding` scaled to unit L2 length.
///
/// Dividing each component by the vector's L2 norm makes euclidean distance over
/// the result rank identically to cosine similarity, which is what lets the
/// euclidean-only [`nearest`](rubix_query::nearest) search serve as semantic
/// recall. An already-normalized vector is returned essentially unchanged.
///
/// # Errors
/// Returns [`AgentError::Embedding`] if `embedding` is empty (no direction to
/// normalize) or has a zero magnitude (a zero vector has no direction, so it
/// cannot be placed on the unit sphere and would never recall meaningfully).
pub fn normalize_embedding(embedding: &[f64]) -> Result<Vec<f64>> {
    if embedding.is_empty() {
        return Err(AgentError::Embedding(
            "an empty embedding has no direction to normalize".to_owned(),
        ));
    }
    let norm = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return Err(AgentError::Embedding(format!(
            "embedding has no usable magnitude (norm = {norm})"
        )));
    }
    Ok(embedding.iter().map(|x| x / norm).collect())
}

#[cfg(test)]
mod tests {
    use super::normalize_embedding;

    /// The L2 norm of a slice, recomputed independently for assertions.
    fn norm(v: &[f64]) -> f64 {
        v.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    #[test]
    fn a_normalized_embedding_has_unit_length() {
        let unit = normalize_embedding(&[3.0, 4.0]).expect("normalize");
        assert!((norm(&unit) - 1.0).abs() < 1e-12);
        // 3-4-5 triangle: the direction is preserved.
        assert!((unit[0] - 0.6).abs() < 1e-12);
        assert!((unit[1] - 0.8).abs() < 1e-12);
    }

    #[test]
    fn normalization_preserves_cosine_ordering_under_euclidean() {
        // Two candidates and a probe. The candidate with the larger cosine
        // similarity to the probe must end up at the smaller euclidean distance
        // once all three are normalized — that equivalence is the whole reason
        // the euclidean-only search can serve as semantic recall.
        let probe = normalize_embedding(&[1.0, 0.0]).expect("probe");
        let near = normalize_embedding(&[2.0, 0.1]).expect("near"); // ~same direction
        let far = normalize_embedding(&[0.0, 5.0]).expect("far"); // orthogonal-ish

        let dist = |a: &[f64], b: &[f64]| {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt()
        };
        assert!(dist(&probe, &near) < dist(&probe, &far));
    }

    #[test]
    fn an_empty_embedding_is_rejected() {
        assert!(normalize_embedding(&[]).is_err());
    }

    #[test]
    fn a_zero_vector_is_rejected() {
        assert!(normalize_embedding(&[0.0, 0.0, 0.0]).is_err());
    }
}
