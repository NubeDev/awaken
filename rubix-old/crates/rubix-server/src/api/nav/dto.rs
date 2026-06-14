//! Nav-route request bodies. The wire shape kept separate from the stored
//! [`rubix_core::NavNode`] so identity (`id`, `org`) is server-assigned on create
//! and immutable on patch.

use rubix_core::{NavContext, NavTarget};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a nav node under `org`. `parent_id` nests it (NULL = root). `context`
/// is only meaningful on a `dashboard` target.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateNavNode {
    pub org: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub title: String,
    #[serde(default)]
    pub sort_order: i64,
    pub target: NavTarget,
    #[serde(default)]
    pub context: Option<NavContext>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
}

/// Patch a nav node. Every field is optional; an absent field is left unchanged.
/// `parent_id`/`sort_order` carry reparent + reorder. `org` is immutable identity
/// and not patchable. A present `context` replaces the stored payload wholesale;
/// to clear it, send an empty object.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PatchNavNode {
    /// Present (including explicit `null` for "move to root") to reparent.
    #[serde(default, deserialize_with = "double_option")]
    pub parent_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub target: Option<NavTarget>,
    #[serde(default, deserialize_with = "double_option")]
    pub context: Option<Option<NavContext>>,
    #[serde(default, deserialize_with = "double_option")]
    pub icon: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub accent: Option<Option<String>>,
}

/// Deserialize a field that distinguishes "absent" (`None`) from "present and
/// null" (`Some(None)`) — needed so a patch can move a node to root (`parent_id:
/// null`) or clear an optional field, versus leaving it untouched.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}
