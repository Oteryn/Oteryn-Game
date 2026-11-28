use crate::qualification::client_world::g17_setup_pending;
use oteryn_protocol_oteryn::daily_reward::returner::Reply;

#[test]
fn g17_setup_accepts_pending_entitlement_on_reused_stack() -> Result<(), Box<dyn std::error::Error>> {
    let id = [
        0x01, 0x93, 0x5f, 0x3b, 0x2c, 0x4d, 0x7a, 0x11, 0x8b, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ];
    assert_eq!(g17_setup_pending(Reply { state: 0, id: None })?, None);
    assert_eq!(
        g17_setup_pending(Reply {
            state: 1,
            id: Some(id),
        })?,
        Some(id)
    );
    assert!(
        g17_setup_pending(Reply {
            state: 2,
            id: Some(id),
        })
        .is_err()
    );
    Ok(())
}
