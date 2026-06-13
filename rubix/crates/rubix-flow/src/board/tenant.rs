//! Derive a board's tenant org from its node configs.
//!
//! Boards are not org-bound documents, but every board that acts on the BMS
//! names points and sites by keyexpr (`{org}/{site}/…`), so the org is implicit
//! in the configs. A `rule` node resolving stored rules needs that org as its
//! tenant scope; this derives it once for the board run, fail-closed (no
//! derivable org → no stored-rule resolution).

use super::schema::BoardGraph;

/// Config keys whose value is a keyexpr (or `{org}/{site}` prefix) the org can
/// be read from: the query/point target and the emit site.
const KEYEXPR_CONFIG_KEYS: [&str; 2] = ["point", "site"];

impl BoardGraph {
    /// The board's tenant org — the first segment of the first keyexpr-bearing
    /// node config, or `None` if the board names no point/site (so a stored-rule
    /// node fails closed rather than guessing a tenant).
    ///
    /// All keyexpr configs in one board address the same `{org}`; the first wins.
    pub fn tenant_org(&self) -> Option<String> {
        self.nodes.iter().find_map(|node| {
            KEYEXPR_CONFIG_KEYS.iter().find_map(|key| {
                node.config
                    .get(*key)
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.split('/').next())
                    .filter(|org| !org.is_empty())
                    .map(|org| org.to_string())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn graph(value: serde_json::Value) -> BoardGraph {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn derives_org_from_a_point_keyexpr() {
        let g = graph(json!({
            "nodes": [{"id": "q", "component": "query_his",
                       "config": {"point": "nube/hq/ahu-3/temp"}}],
            "connections": []
        }));
        assert_eq!(g.tenant_org().as_deref(), Some("nube"));
    }

    #[test]
    fn derives_org_from_an_emit_site() {
        let g = graph(json!({
            "nodes": [{"id": "e", "component": "emit_spark", "config": {"site": "kfc/store-12"}}],
            "connections": []
        }));
        assert_eq!(g.tenant_org().as_deref(), Some("kfc"));
    }

    #[test]
    fn no_keyexpr_config_yields_none() {
        let g = graph(json!({
            "nodes": [{"id": "t", "component": "trigger", "config": {"every": 5}}],
            "connections": []
        }));
        assert_eq!(g.tenant_org(), None);
    }
}
