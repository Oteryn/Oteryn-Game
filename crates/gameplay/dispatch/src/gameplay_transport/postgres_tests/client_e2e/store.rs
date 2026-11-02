use super::super::fixture::{Harness, debug};
use super::TestResult;
use super::support::{self, Case, exchange};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::store as wire;

#[test]
#[ignore = "CLIENT-E2E-1: production Platform Store client is not composed (store: None), crates/gameplay/dispatch/src/gameplay_transport/serving.rs:78; capability 154 is not offered"]
fn client_e2e_store() -> TestResult {
    support::scenario(Case::Store)
}

pub(super) async fn exercise(
    _: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // PlayLink::store_request -> store::command(183, payload); no fake Platform and
    // no synthetic capability offer. Once composed, the first real page must decode.
    let payload = wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
        category: "consumables.exercise-weapons".into(),
        page_cursor: String::new(),
    })
    .map_err(debug)?;
    let page = wire::decode_catalogue_page(
        &exchange(
            owner,
            admitted,
            1,
            wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
            &payload,
        )
        .await?,
    )
    .map_err(debug)?;
    assert!(!page.offers.is_empty());
    Ok(())
}
