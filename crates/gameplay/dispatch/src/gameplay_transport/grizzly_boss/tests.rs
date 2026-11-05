use super::*;

#[test]
fn grizzly_room_catalogue_requires_map_evidence_and_preserves_source_entry_spawn()
-> Result<(), Box<dyn std::error::Error>> {
    let data = quest::catalogue().map_err(|error| format!("{error:?}"))?;
    let mut admitted = 0;
    for boss in &data.bosses {
        if let Some(room) = room(boss.id) {
            assert!(
                room.contains(local(boss.player_position)),
                "{} player",
                boss.name
            );
            assert!(
                room.contains(local(boss.boss_position)),
                "{} boss",
                boss.name
            );
            let mut different_floor = local(boss.player_position);
            different_floor.floor += 1;
            assert!(!room.contains(different_floor));
            admitted += 1;
        } else {
            return Err(format!("missing source-qualified room {}", boss.id).into());
        }
    }
    // W15c admits the debris and ground 9290; rooms 2, 6 and 16 now have map evidence.
    assert_eq!(admitted, 30);
    Ok(())
}

#[test]
fn grizzly_swapped_and_mislocated_source_rectangles_use_verified_map_rooms()
-> Result<(), Box<dyn std::error::Error>> {
    for id in [17, 22, 29, 30] {
        let boss = quest::boss(id).ok_or("boss")?;
        let verified = room(id).ok_or("verified room")?;
        assert!(verified.contains(local(boss.player_position)));
        assert!(
            !verified.contains(local(boss.center)),
            "donor center must be rejected for boss {id}"
        );
    }
    Ok(())
}

#[test]
fn grizzly_room_edges_are_inclusive_and_other_positions_are_outside() {
    let room = Room {
        boss_id: 1,
        qualified: true,
        bounds: Some([100, 200, 110, 210, 8]),
    };
    for (x, y, floor, inside) in [
        (100, 200, 8, true),
        (110, 210, 8, true),
        (99, 200, 8, false),
        (111, 200, 8, false),
        (100, 199, 8, false),
        (100, 211, 8, false),
        (100, 200, 7, false),
    ] {
        assert_eq!(room.contains(MovementLocalPosition { x, y, floor }), inside);
    }
}
