//! The request-time authenticator: turn a bearer string into a [`Principal`].
//! A bearer shaped like a PAT (`rbx_pat_…`) is looked up in the `tokens` store;
//! anything else is verified as an OIDC JWT against the boot-fetched JWKS. Both
//! paths fail closed.

use crate::store::Store;

use super::admin_level::AdminLevel;
use super::error::AuthError;
use super::jwks::JwksVerifier;
use super::pat;
use super::principal::{Principal, Role};
use super::scope::Scope;

/// Verifies bearer tokens. Holds the JWKS verifier (OIDC path) and a store
/// handle (PAT path). Cheap to clone; one per process, carried in `AppState`.
#[derive(Clone)]
pub struct Authenticator {
    jwks: JwksVerifier,
    store: Store,
    /// Bootstrap super-admin subject (`RUBIX_SUPERADMIN_SUBJECT`). When set, this
    /// subject resolves to a global [`Role::Admin`] even without a `users` row.
    superadmin_subject: Option<String>,
}

impl Authenticator {
    pub fn new(jwks: JwksVerifier, store: Store) -> Self {
        Self {
            jwks,
            store,
            superadmin_subject: std::env::var("RUBIX_SUPERADMIN_SUBJECT")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }

    /// Verify a raw bearer string (the value after `Bearer `). Routes a PAT to
    /// the store and everything else to the JWKS verifier, then enriches the
    /// resolved principal with its user identity, team memberships, and admin
    /// tier.
    pub fn verify(&self, bearer: &str) -> Result<Principal, AuthError> {
        let principal = if pat::looks_like_pat(bearer) {
            self.verify_pat(bearer)?
        } else {
            self.jwks.verify(bearer)?
        };
        Ok(self.enrich(principal))
    }

    /// Resolve `principal.subject` to a `users` row and fold the result into the
    /// principal: populate `user_id`/`team_ids` (the Layer-2 grant subjects) and
    /// elevate the role per the user's `admin_level`. The bootstrap super-admin
    /// subject is honored even with no user row, and a first-user fallback makes
    /// the first authenticated subject a super-admin when the table is empty.
    /// A subject with no user row and no bootstrap match keeps its token role —
    /// fully backward compatible with pure-token service accounts.
    fn enrich(&self, mut principal: Principal) -> Principal {
        match self.store.user_by_subject(&principal.subject) {
            Ok(Some(user)) => {
                principal.user_id = Some(user.id);
                principal.team_ids = self.store.team_ids_for_user(user.id).unwrap_or_default();
                match user.admin_level {
                    AdminLevel::SuperAdmin => {
                        principal.role = Role::Admin;
                        principal.scope = Scope::global();
                    }
                    AdminLevel::OrgAdmin => {
                        principal.role = Role::Admin;
                        principal.scope = Scope::org(user.org);
                    }
                    AdminLevel::None => {}
                }
            }
            // No user row: honor the bootstrap super-admin (explicit env, or the
            // first-user fallback on a fresh deployment), else leave as-is.
            _ => {
                if self.is_bootstrap_superadmin(&principal) {
                    principal.role = Role::Admin;
                    principal.scope = Scope::global();
                }
            }
        }
        principal
    }

    /// True when `principal` should be treated as a super-admin without a user
    /// row: it matches the explicit `RUBIX_SUPERADMIN_SUBJECT` bootstrap.
    ///
    /// Bootstrap is intentionally **explicit-only**. An implicit "first caller on
    /// an empty `users` table is super-admin" fallback was considered and
    /// rejected: it would silently elevate every scoped operator on a fresh
    /// deployment (which has no users yet) to global admin, dissolving tenant
    /// confinement until the first user row lands. The env var is reproducible
    /// and carries no such footgun; seed the first admin with it (or with a
    /// pre-provisioned `users` row), not by accident of ordering.
    fn is_bootstrap_superadmin(&self, principal: &Principal) -> bool {
        self.superadmin_subject.as_deref() == Some(principal.subject.as_str())
    }

    /// Verify a PAT: re-hash the presented secret, look the row up, reject a
    /// missing or revoked token.
    fn verify_pat(&self, bearer: &str) -> Result<Principal, AuthError> {
        let (_, secret_hash) =
            pat::parse(bearer).ok_or_else(|| AuthError::InvalidToken("malformed PAT".into()))?;
        let record = self
            .store
            .token_by_hash(&secret_hash)
            .map_err(|e| AuthError::InvalidToken(format!("token lookup failed: {e}")))?
            .ok_or_else(|| AuthError::InvalidToken("unknown PAT".into()))?;
        if !record.is_active() {
            return Err(AuthError::InvalidToken("revoked PAT".into()));
        }
        Ok(record.principal())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use jsonwebtoken::jwk::JwkSet;

    use super::*;
    use crate::auth::{Role, Scope, TokenRecord};

    fn store() -> Store {
        // In-memory store: a fresh temp file per test process is overkill for a
        // PAT round-trip, but `Store::open` needs a path; a tempfile keeps it
        // honest against the real SQLite backend.
        let dir = tempfile::tempdir().expect("tempdir");
        Store::open(&dir.path().join("auth.db")).expect("open store")
    }

    fn authenticator(store: Store) -> Authenticator {
        Authenticator::new(
            JwksVerifier::from_keys(JwkSet { keys: vec![] }, "https://issuer"),
            store,
        )
    }

    #[test]
    fn verifies_a_live_pat_to_its_principal() {
        let store = store();
        let minted = pat::mint();
        store
            .create_token(&TokenRecord {
                id: minted.id.clone(),
                secret_hash: minted.secret_hash.clone(),
                name: "ci".into(),
                role: Role::Service,
                scope: Scope::org("nube"),
                created_at: Utc::now(),
                revoked_at: None,
            })
            .expect("create token");
        let auth = authenticator(store);
        let principal = auth.verify(&minted.plaintext).expect("verify pat");
        assert_eq!(principal.subject, minted.id);
        assert_eq!(principal.role, Role::Service);
        assert_eq!(principal.scope, Scope::org("nube"));
    }

    #[test]
    fn rejects_a_revoked_pat() {
        let store = store();
        let minted = pat::mint();
        store
            .create_token(&TokenRecord {
                id: minted.id.clone(),
                secret_hash: minted.secret_hash.clone(),
                name: "ci".into(),
                role: Role::Service,
                scope: Scope::global(),
                created_at: Utc::now(),
                revoked_at: None,
            })
            .expect("create token");
        store.revoke_token(&minted.id).expect("revoke");
        let auth = authenticator(store);
        assert!(matches!(
            auth.verify(&minted.plaintext),
            Err(AuthError::InvalidToken(_))
        ));
    }

    #[test]
    fn rejects_an_unknown_pat() {
        let auth = authenticator(store());
        let minted = pat::mint();
        assert!(matches!(
            auth.verify(&minted.plaintext),
            Err(AuthError::InvalidToken(_))
        ));
    }
}
