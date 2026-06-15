//! Auth resource routes — login, logout, and the current-principal reflection.
//!
//! The session-issuance surface (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "Auth — close the session-issuance gap"): `POST /auth/login` mints an opaque
//! bearer token, `POST /auth/logout` revokes it, and `GET /auth/me` reflects the
//! authenticated principal and its grants. One file per route; this barrel merges
//! them into a router.

mod login;
mod logout;
mod me;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

use login::login_route;
use logout::logout_route;
use me::me_route;

/// The auth routes mounted under `/auth`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login_route))
        .route("/auth/logout", post(logout_route))
        .route("/auth/me", get(me_route))
}
