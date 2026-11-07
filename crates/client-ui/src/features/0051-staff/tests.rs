use super::*;
use oteryn_client_runtime::raw_feature::{staff as wire, staff_tools::StaffToolsWireError};

#[test]
fn legacy_staff_requests_keep_capability_and_original_signed_payload()
-> Result<(), StaffToolsWireError> {
    let envelope = wire::SignedEnvelope {
        principal: "operator".into(),
        canonical: b" exact signed text ".to_vec(),
        signature: vec![1, 2, 3],
    };
    for (receipt, kind) in [(false, 60), (true, 61)] {
        let command = request(&envelope, receipt)?;
        assert_eq!((command.capability, command.kind), (51, kind));
        assert_eq!(wire::decode_envelope(&command.payload)?, envelope);
    }
    let panel = Panel::default();
    assert!(!panel.selected);
    assert!(!panel.pending);
    assert!(panel.fields.iter().all(String::is_empty));
    Ok(())
}
