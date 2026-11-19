use super::*;
use super::super::item_cosmetics_fixtures::ot_only::*;

#[test]
fn srx4_complete_synthetic_random_pools_pass_the_real_cosmetic_decoder() -> TestResult {
    let bytes = packet()?;
    let content =
        crate::content::native_gameplay::test_support::cosmetics_fixture::decode_cosmetics(&bytes)
            .map_err(debug)?;
    qualify_view(&content)
}
