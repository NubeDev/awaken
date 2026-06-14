//! Integration: the health probe answers on a live, bootstrapped handle.

use rubix_core::RuntimeConfig;
use rubix_store::StoreHandle;

#[tokio::test]
async fn health_returns_ok_on_a_live_handle() {
    let handle = StoreHandle::open(&RuntimeConfig::in_memory("rubix", "health"))
        .await
        .expect("open store");
    handle.health().await.expect("live handle reports healthy");
}
