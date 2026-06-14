//! The dashboard time-range request body and its server-authoritative
//! resolution.
//!
//! Both query paths (`POST /api/v1/query` and `POST /api/v1/datasources/{id}/
//! query`) accept an optional `time_range` + `interval_secs` that the time
//! macros bind against (docs/design/time-range-and-refresh.md §4). The server
//! freezes one `now` per request and resolves the (possibly relative) bounds
//! against it, so a fan-out of widgets in a single dashboard refresh shares one
//! instant with no client/server clock skew (design notes, "Freeze one `now`
//! per refresh"). The resolved [`TimeContext`] flows into the engine; the raw
//! tokens never reach SQL.

use chrono::Utc;
use rubix_query::{resolve_time_range, TimeContext, TimeRangeSpec};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;

/// The wire shape of a dashboard time range: a `from`/`to` pair whose bounds are
/// absolute RFC 3339 instants or relative tokens (`now`, `now-6h`, `now/d`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TimeRangeBody {
    /// Inclusive lower bound: an RFC 3339 instant or a relative token.
    pub from: String,
    /// Exclusive upper bound: an RFC 3339 instant or a relative token.
    pub to: String,
}

/// Resolve an optional request range into a [`TimeContext`] against a freshly
/// frozen server `now`.
///
/// Returns `Ok(None)` when no range was supplied (a query with no time macro is
/// then unaffected, back-compat). A range that fails to resolve (a bad token or
/// an empty `from >= to` range) is a 400 rather than a silent passthrough.
pub fn resolve_request_range(
    body: Option<&TimeRangeBody>,
    interval_secs: Option<u32>,
) -> Result<Option<TimeContext>, ApiError> {
    let Some(body) = body else {
        return Ok(None);
    };
    let spec = TimeRangeSpec {
        from: body.from.clone(),
        to: body.to.clone(),
        interval_secs,
    };
    // One frozen instant for the whole request — every macro resolves against it.
    let now = Utc::now();
    let ctx = resolve_time_range(&spec, now).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Some(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_range_resolves_to_none() {
        let out = resolve_request_range(None, None).unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn relative_range_resolves_against_frozen_now() {
        let body = TimeRangeBody {
            from: "now-6h".into(),
            to: "now".into(),
        };
        let ctx = resolve_request_range(Some(&body), Some(60)).unwrap().unwrap();
        assert_eq!(ctx.interval_secs, 60);
        assert!(ctx.from < ctx.to);
    }

    #[test]
    fn bad_token_is_a_bad_request() {
        let body = TimeRangeBody {
            from: "yesterday".into(),
            to: "now".into(),
        };
        let err = resolve_request_range(Some(&body), None);
        assert!(matches!(err, Err(ApiError::BadRequest(_))));
    }

    #[test]
    fn injection_in_a_bound_is_a_bad_request_not_a_bind() {
        let body = TimeRangeBody {
            from: "'); DROP TABLE his; --".into(),
            to: "now".into(),
        };
        let err = resolve_request_range(Some(&body), None);
        assert!(matches!(err, Err(ApiError::BadRequest(_))));
    }
}
