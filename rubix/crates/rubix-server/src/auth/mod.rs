//! Authentication and RBAC. STACK-DEISGN.md cloud node: "axum: API · auth
//! (OIDC/RBAC)". A request carries an `Authorization: Bearer …` that is either
//! an OIDC JWT (verified against a boot-fetched JWKS) or a PAT / service-account
//! token (looked up in the `tokens` store). Verification yields a [`Principal`]
//! confined to an org→team→site [`Scope`]; per-route gates authorize against the
//! resource's scope.
//!
//! Auth is enforced on the cloud profile and off on edge (`Profile::auth_required`),
//! so local/offline stations keep working unchanged.

pub mod config;
pub mod error;
pub mod gate;
pub mod middleware;
pub mod pat;

mod claims;
mod jwks;
mod principal;
mod scope;
mod token_record;
mod verify;

pub use config::{AuthConfig, ConfigError};
pub use error::AuthError;
pub use gate::RequestPrincipal;
pub use jwks::JwksVerifier;
pub use principal::{Principal, Role};
pub use scope::Scope;
pub use token_record::TokenRecord;
pub use verify::Authenticator;
