//! Private qualification projection; never infer a serving frame from a capability alone.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Basis {
    Native,
    Legacy,
}

impl Basis {
    // Inputs are native positions from the already validated Bundle metadata.
    // Legacy fixtures supply their serving-owner basis explicitly; live callers
    // must obtain the current basis through `current` below.
    pub(super) fn floor(self, native: i8) -> i16 {
        match self {
            Self::Native => i16::from(native),
            Self::Legacy => -i16::from(native),
        }
    }

    pub(super) fn position(self, at: TilePos) -> ActorPosition {
        ActorPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: self.floor(at.floor),
        }
    }
}

fn resolve(selected: bool, marker: Option<u32>, observed_floor: i16) -> TestResult<Basis> {
    // Core spatial projection is independent of capability 385. Absence cannot
    // establish Legacy, and floor zero alone cannot establish either basis.
    let basis = match (selected, marker) {
        (true, Some(2)) => Basis::Native,
        (true, Some(1)) => Basis::Legacy,
        _ => return Err("qualification spatial basis is unproven by current snapshot".into()),
    };
    let consistent = match basis {
        Basis::Native => (-15..=0).contains(&observed_floor),
        Basis::Legacy => (0..=15).contains(&observed_floor),
    };
    if !consistent {
        return Err("qualification spatial position contradicts current basis snapshot".into());
    }
    Ok(basis)
}

pub(super) fn current(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Basis> {
    let observed_floor = client.world_spatial().actor_position.floor;
    let session = client.session_mut();
    let selected = session
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1);
    let marker = session.gated_snapshot(19).map(|row| row.snapshot_type);
    resolve(selected, marker, observed_floor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_native_and_legacy_markers_validate_observed_floor() -> TestResult {
        for floor in [-15, -7, 0] {
            assert_eq!(resolve(true, Some(2), floor)?, Basis::Native);
        }
        for floor in [0, 7, 15] {
            assert_eq!(resolve(true, Some(1), floor)?, Basis::Legacy);
        }
        for (marker, floor) in [(2, 7), (2, -16), (1, -7), (1, 16)] {
            assert!(resolve(true, Some(marker), floor).is_err());
        }
        Ok(())
    }

    #[test]
    fn absent_or_unknown_negotiated_basis_never_defaults_to_legacy() {
        for floor in [-7, 0, 7] {
            for marker in [None, Some(1), Some(2), Some(99)] {
                assert!(resolve(false, marker, floor).is_err());
            }
            assert!(resolve(true, None, floor).is_err());
            assert!(resolve(true, Some(99), floor).is_err());
        }
    }

    #[test]
    fn native_house_native_current_marker_sequence_keeps_each_basis() -> TestResult {
        for (marker, floor, expected) in [(2, -7, -7), (1, 7, 7), (2, -7, -7), (2, 0, 0), (1, 0, 0)]
        {
            let basis = resolve(true, Some(marker), floor)?;
            let native = if floor == 0 { 0 } else { -7 };
            assert_eq!(basis.floor(native), expected);
        }
        Ok(())
    }

    #[test]
    fn fixture_owned_legacy_position_preserves_existing_sparse_wire_coordinates() {
        // Explicit synthetic fixture context, never derived from missing cap385.
        let fixture_basis = Basis::Legacy;
        for floor in [-7, 0] {
            let at = TilePos {
                x: 10,
                y: 20,
                floor,
            };
            assert_eq!(fixture_basis.position(at).floor, -i16::from(floor));
            assert_eq!(Basis::Native.position(at).floor, i16::from(floor));
        }
    }
}
