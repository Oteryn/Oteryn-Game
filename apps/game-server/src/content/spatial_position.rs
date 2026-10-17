//! Canonical opaque placement encoding shared by production and native PG roots.

/// D1241 canonical opaque placement bytes, extracted exactly from death_reward:
/// signed x i32BE || signed y i32BE || floor i16BE. No frame conversion or clamp.
pub fn encode_spatial_position(position: crate::foundation::MovementLocalPosition) -> Vec<u8> {
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&position.x.to_be_bytes());
    spatial_position.extend_from_slice(&position.y.to_be_bytes());
    spatial_position.extend_from_slice(&position.floor.to_be_bytes());
    spatial_position
}

/// Checked conversion into the actual movement carrier. x/y are already i32;
/// LogicalCell z must fit the carrier's i16 floor. Source frame qualification
/// remains the caller's responsibility; this changes neither floor sign nor origin.
pub(crate) fn checked_movement_position(
    cell: super::model::LogicalCell,
) -> Option<crate::foundation::MovementLocalPosition> {
    Some(crate::foundation::MovementLocalPosition {
        x: cell.x,
        y: cell.y,
        floor: i16::try_from(cell.z).ok()?,
    })
}

#[cfg(test)]
mod spatial_position_encoding_tests {
    use super::super::model::LogicalCell;
    use super::{checked_movement_position, encode_spatial_position};

    #[test]
    fn canonical_placement_bytes_preserve_signed_axes_and_check_floor_narrowing()
    -> Result<(), &'static str> {
        let position = checked_movement_position(LogicalCell {
            x: -2,
            y: 0x01020304,
            z: -7,
        })
        .ok_or("valid signed position")?;
        assert_eq!(
            encode_spatial_position(position),
            [0xff, 0xff, 0xff, 0xfe, 1, 2, 3, 4, 0xff, 0xf9]
        );
        for z in [i32::from(i16::MIN), i32::from(i16::MAX)] {
            let position = checked_movement_position(LogicalCell {
                x: i32::MIN,
                y: i32::MAX,
                z,
            })
            .ok_or("floor boundary")?;
            assert_eq!(position.x, i32::MIN);
            assert_eq!(position.y, i32::MAX);
            assert_eq!(i32::from(position.floor), z);
        }
        for z in [
            i32::MIN,
            i32::from(i16::MIN) - 1,
            i32::from(i16::MAX) + 1,
            i32::MAX,
        ] {
            assert!(checked_movement_position(LogicalCell { x: 0, y: 0, z }).is_none());
        }
        Ok(())
    }
}
