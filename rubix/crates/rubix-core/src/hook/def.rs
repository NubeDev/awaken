//! The hook binding — a record that says "on this write, invoke that rule".
//!
//! A hook is a `kind: "hook"` record (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "Server-side hooks"), so binding a side-effect to a write is data, not code,
//! and crosses the gate like any other record. This type is the parsed view of
//! such a record's `content`: which collection's writes it watches
//! (`match`/`content.kind`), which events fire it (`on`), and the rule to invoke
//! (`rule`). Matching is intentionally a pure predicate so the dispatcher can be
//! tested without the engine.

use serde_json::Value;

use super::event::HookEvent;

/// The `kind` value a hook-binding record carries.
pub const HOOK_KIND: &str = "hook";

/// A parsed hook binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    /// The collection kind whose writes this hook watches (`content.kind` of the
    /// changed record).
    pub match_kind: String,
    /// The events that fire this hook.
    pub on: Vec<HookEvent>,
    /// The id of the rule to invoke when the hook fires.
    pub rule: String,
}

/// Why a record's content could not be read as a hook binding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HookParseError {
    /// The content was not a JSON object.
    #[error("hook definition must be a JSON object")]
    NotAnObject,
    /// The `match` field (the watched collection kind) was missing or empty.
    #[error("hook requires a non-empty string `match` (the watched collection kind)")]
    MissingMatch,
    /// The `rule` field (the rule id to invoke) was missing or empty.
    #[error("hook requires a non-empty string `rule` (the rule id to invoke)")]
    MissingRule,
    /// The `on` list was missing, empty, or carried an unknown event.
    #[error("hook requires a non-empty `on` list of create/update/delete")]
    BadEvents,
}

impl Hook {
    /// Read a record's `content` as a hook binding.
    ///
    /// # Errors
    /// Returns a [`HookParseError`] if the content is not an object, or any of
    /// `match`/`rule`/`on` is missing, empty, or malformed.
    pub fn parse(content: &Value) -> Result<Hook, HookParseError> {
        let obj = content.as_object().ok_or(HookParseError::NotAnObject)?;

        let match_kind = obj
            .get("match")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or(HookParseError::MissingMatch)?
            .to_owned();

        let rule = obj
            .get("rule")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or(HookParseError::MissingRule)?
            .to_owned();

        let on = match obj.get("on") {
            Some(Value::Array(entries)) if !entries.is_empty() => entries
                .iter()
                .map(|entry| entry.as_str().and_then(HookEvent::parse))
                .collect::<Option<Vec<_>>>()
                .ok_or(HookParseError::BadEvents)?,
            _ => return Err(HookParseError::BadEvents),
        };

        Ok(Hook { match_kind, on, rule })
    }

    /// Whether this hook fires for `event` on a record of `record_kind`.
    #[must_use]
    pub fn matches(&self, event: HookEvent, record_kind: Option<&str>) -> bool {
        record_kind == Some(self.match_kind.as_str()) && self.on.contains(&event)
    }
}

#[cfg(test)]
mod tests {
    use super::{Hook, HookEvent, HookParseError};
    use serde_json::json;

    fn site_hook() -> Hook {
        Hook::parse(&json!({
            "kind": "hook",
            "match": "site",
            "on": ["create", "update"],
            "rule": "high-temp"
        }))
        .expect("parse")
    }

    #[test]
    fn parses_a_full_binding() {
        let hook = site_hook();
        assert_eq!(hook.match_kind, "site");
        assert_eq!(hook.rule, "high-temp");
        assert_eq!(hook.on, vec![HookEvent::Create, HookEvent::Update]);
    }

    #[test]
    fn matches_only_the_watched_kind_and_events() {
        let hook = site_hook();
        assert!(hook.matches(HookEvent::Create, Some("site")));
        assert!(hook.matches(HookEvent::Update, Some("site")));
        assert!(!hook.matches(HookEvent::Delete, Some("site")));
        assert!(!hook.matches(HookEvent::Create, Some("task")));
        assert!(!hook.matches(HookEvent::Create, None));
    }

    #[test]
    fn missing_fields_are_rejected() {
        assert_eq!(
            Hook::parse(&json!({ "match": "site", "on": ["create"] })),
            Err(HookParseError::MissingRule)
        );
        assert_eq!(
            Hook::parse(&json!({ "rule": "r", "on": ["create"] })),
            Err(HookParseError::MissingMatch)
        );
        assert_eq!(
            Hook::parse(&json!({ "match": "site", "rule": "r", "on": [] })),
            Err(HookParseError::BadEvents)
        );
        assert_eq!(
            Hook::parse(&json!({ "match": "site", "rule": "r", "on": ["nope"] })),
            Err(HookParseError::BadEvents)
        );
    }
}
