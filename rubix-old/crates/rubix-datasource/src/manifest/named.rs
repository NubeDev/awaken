//! An operator-registered named query on a datasource.
//!
//! Named queries are the AI tier's only entry point (docs "Consumers"/"AI"):
//! the SQL stays operator-authored, the caller supplies only the name and bound
//! parameters. `param_count` is the declared positional arity, validated by the
//! executor against the supplied params so an invocation with the wrong number
//! of `$N` values fails before reaching the backend.

use serde::Deserialize;

/// A pre-registered parameterized query invoked by name with bound parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedQuery {
    /// The name a caller invokes.
    pub name: String,
    /// The operator-authored native SQL with `$1..$N` placeholders.
    pub sql: String,
    /// Declared count of positional parameters the SQL expects.
    pub param_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_named_query() {
        let json = r#"{"name":"daily","sql":"SELECT $1::date","param_count":1}"#;
        let q: NamedQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.name, "daily");
        assert_eq!(q.param_count, 1);
    }
}
