//! Gate-side principal persistence.
//!
//! The identity type itself is `rubix_core::Principal` (the crate map owns it in
//! rubix-core). This module owns the principal's persisted shape and the
//! owner-side provisioning verb the gate uses to register identities before they
//! authenticate.

pub(crate) mod row;

mod manage;
mod provision;

pub use manage::{
    create_principal, delete_principal, get_principal, list_principals, set_principal_role,
};
pub use provision::{provision_principal, reprovision_principal};
