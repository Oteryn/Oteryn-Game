//! Capability 16 joins the existing offer sets after journal composition.

use super::*;

#[test]
fn the_production_set_selects_only_offered_capabilities_whatever_the_client_supports()
-> Result<(), Box<dyn Error>> {
    let mut everything: Vec<u32> = Vec::new();
    for capability in registry_capabilities()? {
        everything.push(u32::try_from(capability["id"].as_u64().ok_or("id")?)?);
    }
    everything.extend([2, 3, 4, 5, 9, 11, 12, 13, 14]);
    everything.sort_unstable();
    everything.dedup();
    let production_expected = golden_offer_ids("production")?;
    let item_view_expected = golden_offer_ids("item_view")?;
    assert_eq!(
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&production_expected[..])
    );
    // MAP-ITEM-REF-1: the not-yet-offered Item view set adds 4, which requires 6.
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&item_view_expected[..])
    );
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &[4, 13])
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[13][..])
    );
    Ok(())
}
