//! Validate a rule draft before it crosses the gate.
//!
//! A rule that does not compile, or names an unknown table/grain/aggregate, would
//! fail on its next tick rather than at author time — so the write path validates
//! up front and refuses a broken draft with `400` (`rubix/docs/SCOPE.md`, "Rules
//! fire offline": a stored rule must be runnable). Validation is the same the
//! `rubix-rules` engine applies: [`compile_check`](rubix_rules::compile_check) for
//! the script, and the binding-enum parse for the inputs. The name is a lowercase
//! slug so it is a stable composition handle and a safe record discriminant.

use rubix_rules::compile_check;

use crate::dto::rule::BindingDto;
use crate::error::{ApiError, ApiResult};

/// Reject a name that is not a lowercase slug (`a–z`, `0–9`, hyphen).
///
/// The name is a rule's composition handle and is referenced verbatim by other
/// rules' `invoke`, so it is constrained to a predictable, URL-safe slug.
pub(crate) fn validate_name(name: &str) -> ApiResult<()> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(ApiError::BadRequest(
            "rule name must be a lowercase slug (a–z, 0–9, hyphen)".to_owned(),
        ))
    }
}

/// Validate a rule's script and bindings, mapping any failure to `400`.
///
/// The script must compile under the rule engine, and every binding's
/// table/grain/aggregate must name a known variant — a draft that fails either
/// could never run, so it is refused here rather than stored.
pub(crate) fn validate_definition(script: &str, inputs: &[BindingDto]) -> ApiResult<()> {
    compile_check(script).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    for input in inputs {
        input.to_binding().map_err(|reason| {
            ApiError::BadRequest(format!("binding `{}`: {reason}", input.name))
        })?;
    }
    Ok(())
}
