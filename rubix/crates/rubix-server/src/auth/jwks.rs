//! OIDC JWT verification against a JWKS. The key set is fetched once at boot
//! from the configured `RUBIX_OIDC_JWKS` endpoint and held in memory; each
//! bearer token is matched to a key by its `kid`, its signature verified, and
//! its issuer/expiry validated (STACK-DEISGN.md "validate signature against a
//! configured JWKS/issuer"). A token whose `kid` is absent from the set fails
//! closed — there is no live-registry fallback.

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

use super::claims::Claims;
use super::error::AuthError;
use super::principal::Principal;

/// A boot-fetched JWKS plus the expected issuer. Verifies OIDC JWTs into
/// [`Principal`]s. Cheap to clone (the key set is small); one per process.
#[derive(Clone)]
pub struct JwksVerifier {
    keys: JwkSet,
    issuer: String,
}

impl JwksVerifier {
    /// Fetch the JWKS from `jwks_url` and build a verifier bound to `issuer`.
    /// Run once at boot; a fetch or parse failure aborts startup rather than
    /// leaving auth half-wired.
    pub async fn fetch(jwks_url: &str, issuer: &str) -> anyhow::Result<Self> {
        let body = reqwest::get(jwks_url)
            .await
            .map_err(|e| anyhow::anyhow!("fetch JWKS from {jwks_url}: {e}"))?
            .error_for_status()
            .map_err(|e| anyhow::anyhow!("JWKS endpoint {jwks_url}: {e}"))?
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("read JWKS body from {jwks_url}: {e}"))?;
        let keys: JwkSet = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("parse JWKS from {jwks_url}: {e}"))?;
        if keys.keys.is_empty() {
            anyhow::bail!("JWKS from {jwks_url} contained no keys");
        }
        Ok(Self {
            keys,
            issuer: issuer.to_string(),
        })
    }

    /// Build a verifier from an in-memory key set. Used by tests and by callers
    /// that source the JWKS out of band.
    pub fn from_keys(keys: JwkSet, issuer: &str) -> Self {
        Self {
            keys,
            issuer: issuer.to_string(),
        }
    }

    /// Verify a raw JWT and project it onto a [`Principal`]. Fails closed on an
    /// absent `kid`, an unknown key, a bad signature, a wrong issuer, expiry, or
    /// malformed claims.
    pub fn verify(&self, token: &str) -> Result<Principal, AuthError> {
        let header = decode_header(token).map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::InvalidToken("token header has no kid".into()))?;
        let jwk = self
            .keys
            .find(&kid)
            .ok_or_else(|| AuthError::InvalidToken(format!("no JWKS key for kid `{kid}`")))?;
        let key = DecodingKey::from_jwk(jwk).map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        let mut validation = Validation::new(header.alg);
        // Pin the algorithm family the issuer's keys use; an attacker cannot
        // downgrade to a symmetric alg because the key material is RSA/EC.
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.validate_exp = true;

        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        if !is_asymmetric(header.alg) {
            return Err(AuthError::InvalidToken(format!(
                "unsupported token algorithm {:?}",
                header.alg
            )));
        }
        data.claims
            .into_principal()
            .map_err(AuthError::InvalidToken)
    }
}

/// True for the asymmetric algorithm families a JWKS-backed issuer signs with.
/// Symmetric (HMAC) algorithms are rejected: their "public" key is a shared
/// secret, so accepting one off a JWKS would let a token forger sign their own.
fn is_asymmetric(alg: Algorithm) -> bool {
    matches!(
        alg,
        Algorithm::RS256
            | Algorithm::RS384
            | Algorithm::RS512
            | Algorithm::PS256
            | Algorithm::PS384
            | Algorithm::PS512
            | Algorithm::ES256
            | Algorithm::ES384
            | Algorithm::EdDSA
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_garbage_token() {
        let verifier = JwksVerifier::from_keys(JwkSet { keys: vec![] }, "https://issuer");
        let err = verifier.verify("not.a.jwt").unwrap_err();
        assert!(matches!(err, AuthError::InvalidToken(_)));
    }

    #[test]
    fn hmac_family_is_not_asymmetric() {
        assert!(!is_asymmetric(Algorithm::HS256));
        assert!(is_asymmetric(Algorithm::RS256));
        assert!(is_asymmetric(Algorithm::ES256));
    }
}
