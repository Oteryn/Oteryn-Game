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
    assert_eq!(
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[6, 7, 9, 13, 16, 17, 19, 21, 26, 31, 36, 40, 49, 59][..])
    );
    // MAP-ITEM-REF-1: the not-yet-offered Item view set adds 4, which requires 6.
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[4, 6, 7, 9, 13, 16, 17, 19, 21, 26, 31, 36, 40, 49, 59][..])
    );
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &[4, 13])
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[13][..])
    );
    Ok(())
}
