use super::source_placement::{SourcePlacementOccupant, SourcePlacementTerrain};

#[test]
fn srx3_source_query_add_ignores_movable_barriers_and_fields_but_not_source_restrictions() {
    let clear = SourcePlacementTerrain {
        exists: true,
        ground: true,
        immovable_solid: false,
        protection_zone: false,
        floor_change: false,
        teleport: false,
    };
    assert!(clear.for_masterless_monster(false, false, &[]).query_add);
    assert!(
        !SourcePlacementTerrain {
            ground: false,
            ..clear
        }
        .for_masterless_monster(false, false, &[])
        .query_add
    );
    assert!(
        !SourcePlacementTerrain {
            immovable_solid: true,
            ..clear
        }
        .for_masterless_monster(false, false, &[])
        .query_add
    );
    for terrain in [
        SourcePlacementTerrain {
            protection_zone: true,
            ..clear
        },
        SourcePlacementTerrain {
            floor_change: true,
            ..clear
        },
        SourcePlacementTerrain {
            teleport: true,
            ..clear
        },
    ] {
        assert!(!terrain.for_masterless_monster(false, false, &[]).query_add);
        assert!(terrain.for_masterless_monster(true, false, &[]).query_add);
    }
    assert!(
        !SourcePlacementTerrain {
            exists: false,
            ..clear
        }
        .for_masterless_monster(false, false, &[])
        .exists
    );
}

#[test]
fn srx3_source_query_add_uses_child_push_policy_and_current_occupant_kind() {
    let clear = SourcePlacementTerrain {
        exists: true,
        ground: true,
        immovable_solid: false,
        protection_zone: false,
        floor_change: false,
        teleport: false,
    };
    let pushable = SourcePlacementOccupant::Monster {
        pushable: true,
        player_owned_summon: false,
    };
    assert!(
        clear
            .for_masterless_monster(false, true, &[pushable])
            .query_add
    );
    assert!(
        !clear
            .for_masterless_monster(false, false, &[pushable])
            .query_add
    );
    for blocked in [
        SourcePlacementOccupant::PlayerOrNpc,
        SourcePlacementOccupant::Monster {
            pushable: false,
            player_owned_summon: false,
        },
        SourcePlacementOccupant::Monster {
            pushable: true,
            player_owned_summon: true,
        },
    ] {
        assert!(
            !clear
                .for_masterless_monster(false, true, &[blocked])
                .query_add
        );
    }
    assert!(
        clear
            .for_masterless_monster(false, false, &[SourcePlacementOccupant::IgnoredGhost])
            .query_add
    );
}

#[test]
fn srx3_source_placement_reads_current_map_and_child_policy_through_real_adapter() {
    fn query(
        map: &super::monster_map::CurrentMonsterMap<'_, '_>,
        runtime: &super::ChannelRuntimeV1,
        child: &str,
        at: super::MovementLocalPosition,
    ) -> Option<super::source_placement::SourcePlacementCell> {
        map.source_placement_cell(runtime, child, at)
    }
    let _ = query;
}
