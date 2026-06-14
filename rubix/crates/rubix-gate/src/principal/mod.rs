//! Gate-side principal persistence.
//!
//! The identity type itself is `rubix_core::Principal` (the crate map owns it in
//! rubix-core). This module owns the principal's persisted shape and the
//! owner-side provisioning verb the gate uses to register identities before they
//! authenticate.

pub(crate) mod row;

mod provision;

pub use provision::provision_principal;
