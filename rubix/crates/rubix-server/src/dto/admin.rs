//! Wire shapes for the admin & management surface.
//!
//! The transport DTOs for the control-plane surfaces
//! (`rubix/docs/design/ADMIN-API.md`): principals, grants, tenants, and devices.
//! Like every DTO they are deliberately separate from the gate/domain types so the
//! wire never leaks a secret or a storage-prefixed key — a `PrincipalDto` carries
//! the **API-local** subject, never the `{namespace}_{subject}` storage form, and
//! never the secret.

use rubix_core::{Principal, PrincipalKind, Role};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// A principal as returned to a client — identity only, no secret.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PrincipalDto {
    /// The API-local subject (the `{namespace}_` prefix is stripped).
    pub subject: String,
    /// The namespace (tenant) the principal belongs to.
    pub namespace: String,
    /// Whether the principal is a `user` or an `extension`.
    pub kind: String,
    /// The principal's role band (`viewer`/`operator`/`admin`).
    pub role: String,
}

impl PrincipalDto {
    /// Project a domain principal into its DTO, stripping the namespace prefix
    /// from the storage subject to recover the API-local subject.
    #[must_use]
    pub fn from_principal(principal: &Principal) -> Self {
        Self {
            subject: strip_subject_prefix(&principal.namespace, &principal.subject.to_string()),
            namespace: principal.namespace.clone(),
            kind: kind_str(principal.kind).to_owned(),
            role: role_str(principal.role).to_owned(),
        }
    }
}

/// The body of a create-principal request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePrincipalRequest {
    /// The API-local subject for the new principal (stored namespace-prefixed).
    pub subject: String,
    /// Whether the principal is a `user` or an `extension`.
    pub kind: String,
    /// The principal's role band (`viewer`/`operator`/`admin`).
    pub role: String,
    /// The shared secret the principal will authenticate with. Optional: when
    /// omitted, the server generates one and returns it once in the response
    /// (ADMIN-API open item 5) — the only time a secret crosses the wire.
    #[serde(default)]
    pub secret: Option<String>,
}

/// The response to a create-principal request: the principal plus the secret.
///
/// The secret is returned **only** here, **only** when the server generated it
/// (a caller-supplied secret is echoed back as `None` since the caller already
/// holds it). This is the single response that ever carries a secret — every
/// other principal response is a [`PrincipalDto`] with no secret field.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedPrincipalDto {
    /// The API-local subject.
    pub subject: String,
    /// The namespace (tenant) the principal belongs to.
    pub namespace: String,
    /// Whether the principal is a `user` or an `extension`.
    pub kind: String,
    /// The principal's role band.
    pub role: String,
    /// The generated secret, present only when the server minted it.
    pub secret: Option<String>,
}

impl CreatedPrincipalDto {
    /// Project a principal plus the (optionally generated) secret into the DTO.
    #[must_use]
    pub fn new(principal: &Principal, generated_secret: Option<String>) -> Self {
        let dto = PrincipalDto::from_principal(principal);
        Self {
            subject: dto.subject,
            namespace: dto.namespace,
            kind: dto.kind,
            role: dto.role,
            secret: generated_secret,
        }
    }
}

/// The body of a patch-principal request (role change only).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdatePrincipalRequest {
    /// The new role band (`viewer`/`operator`/`admin`).
    pub role: String,
}

/// A team as returned to a client.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TeamDto {
    /// The team's slug (its stable, API-local key within the namespace).
    pub slug: String,
    /// The namespace the team belongs to.
    pub namespace: String,
    /// A human-readable label for the team.
    pub display_name: String,
}

impl TeamDto {
    /// Project a domain team into its DTO.
    #[must_use]
    pub fn from_team(team: &rubix_gate::Team) -> Self {
        Self {
            slug: team.slug.clone(),
            namespace: team.namespace.clone(),
            display_name: team.display_name.clone(),
        }
    }
}

/// The body of a create-team request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateTeamRequest {
    /// The team's slug (unique within the namespace). Lowercased + trimmed; an
    /// empty slug is rejected by the route.
    pub slug: String,
    /// A human-readable label. Defaults to the slug when omitted.
    #[serde(default)]
    pub display_name: Option<String>,
}

/// The body of an add-member request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    /// The API-local subject of the principal to add (stored namespace-prefixed,
    /// the same convention principals use).
    pub subject: String,
}

/// A team member as returned to a client — the API-local subject.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TeamMemberDto {
    /// The member principal's API-local subject (the `{namespace}_` prefix is
    /// stripped).
    pub subject: String,
}

/// A capability grant as returned to a client.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GrantDto {
    /// The API-local subject the grant is attached to.
    pub subject: String,
    /// The namespace the grant is confined to.
    pub namespace: String,
    /// The capability, as its stable wire string.
    pub capability: String,
}

/// A tenant (onboarded namespace) as returned to a client.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantDto {
    /// The tenant id (the namespace suffix).
    pub id: String,
    /// The full namespace the tenant resolved to (`tenant_{id}`).
    pub namespace: String,
    /// When the tenant was onboarded (RFC 3339, UTC).
    pub created_at: String,
}

/// The body of a create-tenant (onboarding) request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    /// The tenant id to onboard (resolves to namespace `tenant_{id}`).
    pub id: String,
    /// The API-local subject of the tenant's first admin.
    pub admin_subject: String,
    /// The secret the first admin authenticates with.
    pub admin_secret: String,
}

/// A device registry entry as returned to a client.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeviceDto {
    /// The API-local device id (the `{namespace}_` prefix is stripped).
    pub id: String,
    /// The namespace the device belongs to.
    pub namespace: String,
    /// A human label for the device.
    pub label: String,
    /// The device class (free-form, e.g. `gateway`, `sensor`).
    pub kind: String,
    /// An open key/value metadata bag (no fixed schema).
    pub metadata: BTreeMap<String, Value>,
}

/// The body of a create-device request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDeviceRequest {
    /// The API-local device id (unique within the namespace).
    pub id: String,
    /// A human label for the device.
    pub label: String,
    /// The device class (free-form).
    pub kind: String,
    /// An open key/value metadata bag.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// The body of a patch-device request — every field optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDeviceRequest {
    /// A new label, if changing.
    pub label: Option<String>,
    /// A new device class, if changing.
    pub kind: Option<String>,
    /// A replacement metadata bag, if changing.
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Build the full `{namespace}_{subject}` storage subject from an API-local one.
///
/// The `principal`/grant tables are keyed globally, so each tenant's subject space
/// is carved out by a namespace prefix — the same convention the seed uses
/// (`rubix/docs/design/ADMIN-API.md`, "Subject keying").
#[must_use]
pub fn prefix_subject(namespace: &str, local: &str) -> String {
    format!("{namespace}_{local}")
}

/// Recover the API-local subject from a full storage subject.
///
/// Strips the leading `{namespace}_` prefix; a subject that does not carry it
/// (defensive — e.g. legacy data) is returned unchanged.
#[must_use]
pub fn strip_subject_prefix(namespace: &str, full: &str) -> String {
    let prefix = format!("{namespace}_");
    full.strip_prefix(&prefix).unwrap_or(full).to_owned()
}

/// Parse a `user`/`extension` wire string into a [`PrincipalKind`].
///
/// # Errors
/// Returns the offending string when it is neither known kind.
pub fn parse_kind(raw: &str) -> Result<PrincipalKind, String> {
    match raw {
        "user" => Ok(PrincipalKind::User),
        "extension" => Ok(PrincipalKind::Extension),
        other => Err(other.to_owned()),
    }
}

/// Parse a `viewer`/`operator`/`admin` wire string into a [`Role`].
///
/// # Errors
/// Returns the offending string when it is not a known role.
pub fn parse_role(raw: &str) -> Result<Role, String> {
    match raw {
        "viewer" => Ok(Role::Viewer),
        "operator" => Ok(Role::Operator),
        "admin" => Ok(Role::Admin),
        other => Err(other.to_owned()),
    }
}

/// The wire string for a principal kind (matches the serialized domain form).
fn kind_str(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::User => "user",
        PrincipalKind::Extension => "extension",
    }
}

/// The wire string for a role band (matches the serialized domain form).
fn role_str(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Operator => "operator",
        Role::Admin => "admin",
    }
}

#[cfg(test)]
mod tests {
    use super::{prefix_subject, strip_subject_prefix};

    #[test]
    fn prefix_and_strip_round_trip() {
        let full = prefix_subject("tenant_acme", "alice");
        assert_eq!(full, "tenant_acme_alice");
        assert_eq!(strip_subject_prefix("tenant_acme", &full), "alice");
    }

    #[test]
    fn strip_leaves_an_unprefixed_subject_unchanged() {
        assert_eq!(strip_subject_prefix("tenant_acme", "root"), "root");
    }

    #[test]
    fn strip_handles_a_subject_containing_the_separator() {
        // A local subject with an underscore survives a round trip: only the
        // leading namespace prefix is stripped, not every underscore.
        let full = prefix_subject("tenant_acme", "edge_01");
        assert_eq!(strip_subject_prefix("tenant_acme", &full), "edge_01");
    }
}
