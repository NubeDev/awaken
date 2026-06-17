//! Print the OpenAPI 3.1 document as pretty JSON to stdout.
//!
//! The document is normally served at `GET /api-docs/openapi.json` (WS-16), but
//! the static doc-site needs the spec at build time without booting the server.
//! This example dumps the same [`rubix_server::openapi_document`] so the docs can
//! generate a static `openapi.json` and render it with an in-browser viewer.
//!
//! Run with: `cargo run -p rubix-server --example dump_openapi`

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = rubix_server::openapi_document();
    println!("{}", doc.to_pretty_json()?);
    Ok(())
}
