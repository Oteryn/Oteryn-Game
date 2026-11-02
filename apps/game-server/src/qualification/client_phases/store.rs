//! Platform Store over the real node; reference: `client_e2e/store.rs`.
use super::*;
pub(super) async fn store(_client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    // serve.rs composes StoreClient and selects 154 on the node, but node_boot/run.sh pins
    // Platform 3bc2d0e9, which has no Store API. Its wp5_s3b nginx proxy also exposes only
    // native-evidence and character-bootstrap-intents/read. A selected Game capability
    // cannot supply the missing Platform balance route required by catalogue_page.
    evidence("mechanic=store skipped=pinned_platform_has_no_store_api");
    Ok(())
}
