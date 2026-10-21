//! Source field identity conversion and safe-world eligibility, without Item authority.
pub fn select_safe_field(
    source_id: u32,
    world_no_pvp: bool,
    target_no_pvp: Option<bool>,
) -> Result<u32, &'static str> {
    let no_pvp = target_no_pvp.ok_or("field tile PvP policy unknown")?;
    // ATTACK-1b already holds disconnected actors using its current in-fight owner.
    // Field creation/hits are not connected to that owner, and FIELD-1 excludes
    // player-affecting PvP. Keep this slice restricted to qualified NoPvP contexts.
    if !no_pvp && !world_no_pvp {
        return Err("field in-fight owner required");
    }
    // Canary999025 combat.cpp1189–1244 and utils_definitions.hpp589–603.
    Ok(select_source_field_id(
        source_id,
        true,
        no_pvp,
        world_no_pvp,
    ))
}

/// Shared exact source Item identity normalization; current World/tile policy remains mandatory.
/// Pure helper grants no creature timer or durable Item authority.
pub fn select_source_field_id(
    source_id: u32,
    player_owned: bool,
    no_pvp: bool,
    world_no_pvp: bool,
) -> u32 {
    let pvp = match source_id {
        2123 => 2118,
        2124 => 2119,
        2125 => 2120,
        2126 => 2122,
        2121 => 105,
        other => other,
    };
    if player_owned && (no_pvp || world_no_pvp) {
        match pvp {
            2118 => 21465,
            105 => 2134,
            2122 => 2135,
            other => other,
        }
    } else {
        pvp
    }
}
#[cfg(test)]
mod source_field_selection_tests {
    use super::*;
    #[test]
    fn source_field_ids_apply_safe_world_conversion_only_to_actual_player_owned_casters() {
        for (source, safe) in [(2118, 21465), (2122, 2135), (105, 2134)] {
            assert_eq!(select_source_field_id(source, false, true, true), source);
            assert_eq!(select_source_field_id(source, true, false, false), source);
            assert_eq!(select_source_field_id(source, true, true, false), safe);
            assert_eq!(select_source_field_id(source, true, false, true), safe);
        }
        assert_eq!(select_source_field_id(2123, true, false, false), 2118);
        assert_eq!(select_source_field_id(2123, true, true, false), 21465);
    }
}

#[cfg(test)]
mod safe_field_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn qualified_safe_context_keeps_source_conversion_and_decay_ids() {
        for (source, selected) in [
            (2123, 21465),
            (2118, 21465),
            (2121, 2134),
            (105, 2134),
            (2126, 2135),
            (2122, 2135),
            (2124, 2119),
            (2125, 2120),
            (2119, 2119),
            (2120, 2120),
        ] {
            assert_eq!(
                select_safe_field(source, true, Some(false)).unwrap(),
                selected
            );
            assert_eq!(
                select_safe_field(source, false, Some(true)).unwrap(),
                selected
            );
            assert_eq!(
                select_safe_field(source, false, Some(true)).unwrap(),
                selected
            );
        }
    }

    #[test]
    fn losing_safe_tile_or_changing_world_context_cannot_admit_pvp_creation() {
        for source in [2123, 2121, 2126] {
            assert!(select_safe_field(source, false, Some(true)).is_ok());
            assert!(select_safe_field(source, false, Some(false)).is_err());
            assert!(select_safe_field(source, false, Some(false)).is_err());
            assert!(select_safe_field(source, true, Some(false)).is_ok());
        }
    }

    #[test]
    fn unknown_tile_policy_is_unavailable_even_in_a_no_pvp_world() {
        for mode in [true, false, false] {
            assert!(select_safe_field(2123, mode, None).is_err());
        }
    }
}
