//! Player-trade request construction concern.
// Called through the registration entry named by the adjacent JSON.
#[allow(dead_code)]
pub fn request(
    panel: &mut super::PlayerTradePanel,
    intent: super::Intent,
) -> Option<super::PlayerTradeCommand> {
    super::request(panel, intent)
}
