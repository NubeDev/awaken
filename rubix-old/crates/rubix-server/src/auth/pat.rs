//! Personal access tokens / service-account credentials. A PAT is an opaque
//! bearer string `rbx_pat_<id>.<secret>` presented in place of an OIDC JWT
//! (STACK-DEISGN.md "PATs and service accounts for machine access"). The store
//! holds only the SHA-256 of the secret, never the secret itself: issuing
//! returns the plaintext once, and verification re-hashes the presented secret
//! to look the row up. The `<id>` prefix scopes the hash lookup so a lookup is a
//! single indexed read, not a scan over every token.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The fixed prefix that marks a bearer string as a PAT rather than a JWT.
pub const PAT_PREFIX: &str = "rbx_pat_";

/// A freshly minted token: the public `id` (persisted) and the one-time
/// `plaintext` handed to the caller. The secret is never stored — only its
/// [`hash`].
pub struct MintedToken {
    /// Public token id, also the `subject` of the resulting principal.
    pub id: String,
    /// The full bearer string to present. Shown once at issue time.
    pub plaintext: String,
    /// SHA-256 of the secret half, the only thing persisted.
    pub secret_hash: String,
}

/// Mint a new PAT. The id is a UUID; the secret is a second UUID's simple form,
/// giving 122 bits of entropy without pulling in a separate RNG dependency
/// (`uuid` already seeds from the OS CSPRNG).
pub fn mint() -> MintedToken {
    let id = Uuid::new_v4().simple().to_string();
    let secret = Uuid::new_v4().simple().to_string();
    let plaintext = format!("{PAT_PREFIX}{id}.{secret}");
    MintedToken {
        secret_hash: hash(&secret),
        id,
        plaintext,
    }
}

/// True when a bearer string is shaped like a PAT (so the verifier routes it to
/// the PAT path rather than the JWT path).
pub fn looks_like_pat(bearer: &str) -> bool {
    bearer.starts_with(PAT_PREFIX)
}

/// Split a presented PAT into its `(id, secret_hash)`. Returns `None` for a
/// malformed string so verification fails closed.
pub fn parse(bearer: &str) -> Option<(String, String)> {
    let body = bearer.strip_prefix(PAT_PREFIX)?;
    let (id, secret) = body.split_once('.')?;
    if id.is_empty() || secret.is_empty() {
        return None;
    }
    Some((id.to_string(), hash(secret)))
}

/// SHA-256 of a secret, hex-encoded. The same function hashes at issue time and
/// at verify time, so the stored hash and the lookup hash match byte for byte.
pub fn hash(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex(&digest)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minted_token_round_trips_through_parse() {
        let minted = mint();
        assert!(looks_like_pat(&minted.plaintext));
        let (id, secret_hash) = parse(&minted.plaintext).expect("parse minted");
        assert_eq!(id, minted.id);
        assert_eq!(secret_hash, minted.secret_hash);
    }

    #[test]
    fn distinct_mints_have_distinct_secrets() {
        let a = mint();
        let b = mint();
        assert_ne!(a.id, b.id);
        assert_ne!(a.secret_hash, b.secret_hash);
    }

    #[test]
    fn malformed_pats_fail_closed() {
        assert!(parse("rbx_pat_only-id").is_none());
        assert!(parse("rbx_pat_.secret").is_none());
        assert!(parse("rbx_pat_id.").is_none());
        assert!(parse("bearer-jwt").is_none());
        assert!(!looks_like_pat("eyJhbGci.payload.sig"));
    }

    #[test]
    fn hash_is_stable_and_distinguishing() {
        assert_eq!(hash("abc"), hash("abc"));
        assert_ne!(hash("abc"), hash("abd"));
        // Known SHA-256 of "abc".
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
