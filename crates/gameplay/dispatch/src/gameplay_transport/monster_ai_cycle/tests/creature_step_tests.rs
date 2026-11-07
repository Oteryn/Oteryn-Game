use super::*;

/// The accepted room's rat goals are searched and committed on their own deadlines. The den
/// rat moves west; the den-north rat cannot enter a door or the occupied southern cell.
#[test]
#[allow(clippy::expect_used)]
fn a_spawned_rat_commits_a_chase_step_in_the_accepted_entry_room() {
    use {
        crate::foundation::{ChannelId, NodeId, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };
    let world =
        WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("world");
    let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
        .expect("channel");
    let node =
        NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
    let (room, door) = entry_room_and_door(world, channel);
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (0, 0, 0),
        ),
    )
    .expect("runtime");
    let source = room.spawn_source().expect("D116 spawn source");
    assert_eq!(
        runtime.realize_activation_spawns(vec![source.activation_facts()]),
        Ok(2)
    );
    let rat_at = |runtime: &ChannelRuntimeV1, at: MovementLocalPosition| {
        runtime
            .spawn_point_creatures()
            .into_iter()
            .find_map(|(_, _, live)| {
                live.filter(|rat| {
                    runtime
                        .read_actor_position(*rat)
                        .is_ok_and(|p| p.position() == at)
                })
            })
            .expect("a spawned rat on the cell")
    };
    let den = rat_at(&runtime, cell(2, 0));
    let den_north = rat_at(&runtime, cell(2, -1));
    let session = GameSessionId::decode(&[1, 0, 0, 0, 0, 4, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 4])
        .expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reservation).expect("player");
    runtime
        .initialize_pinned_test_position(player, cell(0, 0))
        .expect("player on the entry start");

    // The room's own hostile rat behaviour, its swing cleared as the AI pass clears it.
    let room_source: serde_json::Value =
        serde_json::from_slice(crate::content::NATIVE_ENTRY_ROOM_SOURCE).expect("room source");
    let profile = room_source["authoring_profiles"]
        .as_array()
        .expect("authoring profiles")
        .iter()
        .find(|profile| profile["target"]["key"] == "oteryn:behavior/rat-hostile")
        .expect("rat behaviour");
    let mut behavior: ProjectV2BehaviorAuthoring =
        serde_json::from_value(profile["data"]["profile"].clone()).expect("behaviour");
    behavior.attacks.clear();
    behavior.defenses.clear();
    let mut table = crate::ai_think::CreatureAiTable::default();
    for rat in [den, den_north] {
        table
            .admit_creature(
                &runtime,
                rat,
                Some(&behavior),
                &std::collections::BTreeMap::new(),
                0,
            )
            .expect("admitted");
    }
    let candidates = [TargetCandidate {
        actor: player,
        position: cell(0, 0),
        eligible: true,
        invisible: false,
        health: 0,
        damage: 0,
    }];
    let revisions = crate::ability::RevisionSet::new(
        "ruleset:ai-v1",
        "content:test",
        "world:ai-v1",
        "formula:source-melee-baseline",
        "simulation:v1",
    )
    .expect("revisions");
    let doors = door.collision_cells();
    let cells = room.movement_cells();
    let reports = table.run_due_thinks(
        &mut runtime,
        0,
        &GameplayDecisionRoot::from_bytes([9; 32]),
        &revisions,
        &candidates,
    );
    assert_eq!(reports.len(), 2, "both rats thought");
    assert!(reports.iter().all(|(_, report)| matches!(
        report.goal,
        crate::ai_think::movement_goal::MovementGoal::Chase { .. }
    )));
    assert_eq!(
        runtime
            .read_actor_position(den)
            .expect("before step")
            .position(),
        cell(2, 0)
    );
    let sources: Vec<_> = [den, den_north]
        .into_iter()
        .map(
            |actor| crate::movement::speed::creature_speed_fixture::Native {
                actor,
                base: 67,
                pin: runtime.content_pin().server_artifact_digest(),
            },
        )
        .collect();
    let mut budget = crate::movement::path_budget::ChannelPathBudget::default();
    let next = table
        .drive_movement(
            &mut runtime,
            cells,
            doors,
            room.map_revision_digest(),
            &sources,
            &crate::movement::speed::EngineeringGroundSpeed,
            &GameplayDecisionRoot::from_bytes([9; 32]),
            0,
            &mut budget,
            false,
        )
        .expect("step deadline");
    table.drive_movement(
        &mut runtime,
        cells,
        doors,
        room.map_revision_digest(),
        &sources,
        &crate::movement::speed::EngineeringGroundSpeed,
        &GameplayDecisionRoot::from_bytes([9; 32]),
        next,
        &mut budget,
        false,
    );
    assert_eq!(
        runtime
            .read_actor_position(den)
            .expect("den rat")
            .position(),
        cell(1, 0),
        "the step was committed"
    );
    assert_eq!(
        runtime
            .read_actor_position(den_north)
            .expect("den-north rat")
            .position(),
        cell(2, -1)
    );
    let after = runtime.read_actor_position(den).expect("moved creature");
    let moved =
        crate::movement::relocate::relocate_creature(&mut runtime, den, cells, doors, cell(2, 0))
            .expect("relocation");
    assert_eq!(moved.revision(), after.revision() + 1);
    for destination in [cell(0, 0), cell(1, -1), cell(9, 9)] {
        assert!(
            crate::movement::relocate::relocate_creature(
                &mut runtime,
                den,
                cells,
                doors,
                destination
            )
            .is_err()
        );
        assert_eq!(runtime.read_actor_position(den).expect("unchanged"), moved);
    }
    assert!(
        crate::movement::relocate::relocate_creature(
            &mut runtime,
            player,
            cells,
            doors,
            cell(1, 0)
        )
        .is_err()
    );
}
