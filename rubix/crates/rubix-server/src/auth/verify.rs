//! The request-time authenticator: turn a bearer string into a [`Principal`].
//! A bearer shaped like a PAT (`rbx_pat_…`) is looked up in the `tokens` store;
//! anything else is verified as an OIDC JWT against the boot-fetched JWKS. Both
//! paths fail closed.

use crate::store::Store;

use super::error::AuthError;
use super::jwks::JwksVerifier;
use super::pat;
use super::principal::Principal;

/// Verifies bearer tokens. Holds the JWKS verifier (OIDC path) and a store
/// handle (PAT path). Cheap to clone; one per process, carried in `AppState`.
#[derive(Clone)]
pub struct Authenticator {
    jwks: JwksVerifier,
    store: Store,
}

impl Authenticator {
    pub fn new(jwks: JwksVerifier, store: Store) -> Self {
        Self { jwks, store }
    }

    /// Verify a raw bearer string (the value after `Bearer `). Routes a PAT to
    /// the store and everything else to the JWKS verifier.
    pub fn verify(&self, bearer: &str) -> Result<Principal, AuthError> {
        if pat::looks_like_pat(bearer) {
            self.verify_pat(bearer)
        } else {
            self.jwks.verify(bearer)
        }
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
