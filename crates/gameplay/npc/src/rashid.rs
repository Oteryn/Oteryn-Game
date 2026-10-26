//! Global Rashid schedule and Travelling Trader gates.
//!
//! Weekdays are derived from the accepted server-save boundary by the world owner.  This module
//! deliberately accepts that day ordinal rather than consulting the host clock.

/// Durable quest track which unlocks Rashid's shop after Sunday's recognition.
pub const MISSION_07_TRACK: &str =
    "canary:quest-progress/quest/u8_1/the_travelling_trader/mission07";

/// Rashid's city for a server-save day (`0` is Monday, `6` is Sunday).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RashidCity {
    Svargrond,
    LibertyBay,
    PortHope,
    Ankrahmun,
    Darashia,
    Edron,
    Carlin,
}

/// Resolve placement from the accepted server-save day, never the host's local weekday.
#[must_use]
pub const fn city_after_server_save(day_since_monday: u8) -> Option<RashidCity> {
    match day_since_monday {
        0 => Some(RashidCity::Svargrond),
        1 => Some(RashidCity::LibertyBay),
        2 => Some(RashidCity::PortHope),
        3 => Some(RashidCity::Ankrahmun),
        4 => Some(RashidCity::Darashia),
        5 => Some(RashidCity::Edron),
        6 => Some(RashidCity::Carlin),
        _ => None,
    }
}

/// Global's final trade gate. Missing/unloaded quest state fails closed.
#[must_use]
pub fn trade_unlocked(track_value: Option<i64>) -> bool {
    track_value == Some(1)
}

/// Both donor spellings identify the same NPC service owner.
#[must_use]
pub fn is_rashid(npc_key: &str) -> bool {
    npc_key.rsplit(['/', '.']).next() == Some("rashid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_weekday_order_uses_server_save_ordinal() {
        assert_eq!(city_after_server_save(0), Some(RashidCity::Svargrond));
        assert_eq!(city_after_server_save(1), Some(RashidCity::LibertyBay));
        assert_eq!(city_after_server_save(2), Some(RashidCity::PortHope));
        assert_eq!(city_after_server_save(3), Some(RashidCity::Ankrahmun));
        assert_eq!(city_after_server_save(4), Some(RashidCity::Darashia));
        assert_eq!(city_after_server_save(5), Some(RashidCity::Edron));
        assert_eq!(city_after_server_save(6), Some(RashidCity::Carlin));
        assert_eq!(city_after_server_save(7), None);
    }

    #[test]
    fn only_sunday_recognition_unlocks_trade() {
        assert!(!trade_unlocked(None));
        assert!(!trade_unlocked(Some(0)));
        assert!(trade_unlocked(Some(1)));
        assert!(!trade_unlocked(Some(2)));
    }

    #[test]
    fn rashid_identity_does_not_gate_other_traders() {
        assert!(is_rashid("canary:npc/rashid"));
        assert!(is_rashid("oteryn:npc.rashid"));
        assert!(!is_rashid("canary:npc/rashid_custom"));
        assert!(!is_rashid("canary:npc/nah'bob"));
    }
}
