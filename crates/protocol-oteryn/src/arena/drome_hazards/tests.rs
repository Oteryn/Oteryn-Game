use super::*;
#[test]
fn srx3_drome_selected_modifier_codec_survives_without_active_marks() -> Result<(), ArenaWireError>
{
    let selected = DromeHazards {
        selected: [1, 4],
        ..Default::default()
    };
    let payload = append_drome_hazards(
        DromeResult {
            disposition: 1,
            phase: 1,
            ..Default::default()
        }
        .encode(),
        &selected,
    )?;
    assert_eq!(decode_drome_hazards(&payload)?.selected, [1, 4]);
    Ok(())
}
#[test]
fn srx3_drome_selected_modifier_codec_refuses_partial_duplicate_and_unknown_pairs() {
    for pair in [[1, 1], [0, 4], [1, 10], [1, 0]] {
        assert!(
            append_drome_hazards(
                DromeResult {
                    disposition: 1,
                    phase: 1,
                    ..Default::default()
                }
                .encode(),
                &DromeHazards {
                    selected: pair,
                    ..Default::default()
                },
            )
            .is_err(),
            "invalid selected modifier pair {pair:?} was accepted"
        );
    }
}
fn snapshot() -> DromeHazards {
    DromeHazards {
        selected: [0, 0],
        observed_at_micros: 15_000_000,
        entries: vec![
            DromeHazard {
                kind: DromeHazardKind::Lava,
                expires_at_micros: 18_000_000,
                positions: (0..100)
                    .map(|index| crate::world_spatial::ActorPosition {
                        x: 32300 + index % 10,
                        y: 31100 + index / 10,
                        floor: -8,
                    })
                    .collect(),
            },
            DromeHazard {
                kind: DromeHazardKind::Sorrow,
                expires_at_micros: 21_000_000,
                positions: vec![crate::world_spatial::ActorPosition {
                    x: 32303,
                    y: 31103,
                    floor: -8,
                }],
            },
        ],
    }
}
#[test]
fn srx3_drome_hazard_codec_preserves_legacy_and_roundtrips_exact_source_deadlines()
-> Result<(), ArenaWireError> {
    let result = DromeResult {
        disposition: 1,
        phase: 2,
        wave: 0,
        ..Default::default()
    };
    let legacy = result.encode();
    assert_eq!(
        append_drome_hazards(legacy.clone(), &DromeHazards::default())?,
        legacy
    );
    let snapshot = snapshot();
    let bytes = append_drome_hazards(legacy, &snapshot)?;
    assert_eq!(DromeResult::decode(&bytes)?, result);
    assert_eq!(decode_drome_hazards(&bytes)?, snapshot);
    Ok(())
}
#[test]
fn srx3_drome_hazard_codec_allows_200_across_two_distinct_mark_kinds() -> Result<(), ArenaWireError>
{
    let mut snapshot = snapshot();
    snapshot.entries[1] = DromeHazard {
        kind: DromeHazardKind::Beam,
        expires_at_micros: 18_000_000,
        positions: snapshot.entries[0].positions.clone(),
    };
    let bytes = append_drome_hazards(
        DromeResult {
            disposition: 1,
            ..Default::default()
        }
        .encode(),
        &snapshot,
    )?;
    assert_eq!(decode_drome_hazards(&bytes)?, snapshot);
    Ok(())
}
#[test]
fn srx3_drome_hazard_codec_refuses_duplicate_tiles_expired_and_unbounded_marks() {
    let mut snapshot = snapshot();
    snapshot.entries[0].positions[1] = snapshot.entries[0].positions[0];
    assert!(
        append_drome_hazards(
            DromeResult {
                disposition: 1,
                ..Default::default()
            }
            .encode(),
            &snapshot
        )
        .is_err()
    );
    let mut snapshot = self::snapshot();
    snapshot.entries[0].expires_at_micros = 15_000_000;
    assert!(
        append_drome_hazards(
            DromeResult {
                disposition: 1,
                ..Default::default()
            }
            .encode(),
            &snapshot
        )
        .is_err()
    );
    let mut snapshot = self::snapshot();
    snapshot.entries[0]
        .positions
        .push(crate::world_spatial::ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        });
    assert!(
        append_drome_hazards(
            DromeResult {
                disposition: 1,
                ..Default::default()
            }
            .encode(),
            &snapshot
        )
        .is_err()
    );
}
