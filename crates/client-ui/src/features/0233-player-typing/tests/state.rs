use super::*;
use oteryn_client_runtime::player_typing;

#[test]
fn server_wire_updates_add_and_clear_the_exact_actor_generation() -> Result<(), player_typing::Error>
{
    let bytes = [
        10, 16, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 16, 9, 24, 1,
    ];
    let mut panel = PlayerTypingPanel::default();
    protocol::reduce(&mut panel, player_typing::decode_update(&bytes)?);
    assert_eq!(panel.indicators.len(), 1);
    assert_eq!(panel.indicators[0].identity, [7; 16]);
    let mut off = bytes;
    off[21] = 0;
    protocol::reduce(&mut panel, player_typing::decode_update(&off)?);
    assert!(panel.indicators.is_empty());
    Ok(())
}
