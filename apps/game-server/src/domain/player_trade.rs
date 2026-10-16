//! PLAYER_TRADE0 leaf policy and native offer data, not admission or a reservation proof.
#![allow(
    dead_code,
    reason = "private Trade component awaits physical reservations and atomic writer integration"
)]

use crate::content::{ReferenceItemField, ReferenceItemTradeRestrictions};
/// Unknown direct-trade permission is explicitly allowed; marketable is not a substitute.
/// This checks only the flag, not the other offer/current-authority requirements.
pub(crate) fn tradeable_flag_allows(restrictions: &ReferenceItemTradeRestrictions) -> bool {
    matches!(
        restrictions.tradeable,
        ReferenceItemField::Unknown | ReferenceItemField::Known(true)
    )
}
