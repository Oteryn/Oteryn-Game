use super::*;

#[test]
fn walk_back_reaches_spawn_and_idles_while_a_failed_route_waits_for_wake() {
    for walkable in [true, false] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(90, 100), 0);
        let spawn = at(86, 100);
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .spawn = Some(spawn);
        let reports = channel.thinks(0, walkable);
        assert!(!reports[0].1.idle, "the walk-back goal retains its think");
        for _ in 0..8 {
            let Some(due) = channel.table.entry(creature).expect("entry").motion.due else {
                break;
            };
            channel.advance_movement(due, walkable);
        }
        assert_eq!(
            channel.position(creature),
            if walkable { spawn } else { at(90, 100) }
        );
        if walkable {
            let due = channel.table.due(creature).expect("idle think due");
            assert!(channel.thinks(due, walkable)[0].1.idle);
        }
        assert_eq!(channel.table.due(creature), None);
        assert!(
            channel
                .table
                .entry(creature)
                .expect("entry")
                .motion
                .due
                .is_none()
        );
    }
}

#[test]
fn leash_relocates_once_or_idles_in_place_if_the_spawn_is_occupied() {
    for occupied in [false, true] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(50, 100), 0);
        let spawn = if occupied {
            channel.position(channel.player)
        } else {
            at(105, 100)
        };
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .spawn = Some(spawn);
        let before = channel
            .runtime
            .read_actor_position(creature)
            .expect("before");
        channel.thinks(0, true);
        let after = channel
            .runtime
            .read_actor_position(creature)
            .expect("after");
        assert_eq!(
            after.position(),
            if occupied { before.position() } else { spawn }
        );
        assert_eq!(after.revision(), before.revision() + u64::from(!occupied));
        assert_eq!(channel.table.due(creature), None);
        assert!(
            channel
                .table
                .entry(creature)
                .expect("entry")
                .motion
                .due
                .is_none()
        );
    }
}

#[test]
fn fire_overlord_walk_back_rechecks_targets_before_clearing_damage() {
    assert_fire_overlord_returns(&[
        (at(114, 100), true, at(103, 100), true),
        (at(115, 100), true, at(104, 100), true),
        (at(116, 100), true, at(104, 100), false),
    ]);
}

#[test]
fn fire_overlord_walk_back_stays_active_for_visible_non_attackable_players() {
    assert_fire_overlord_returns(&[
        (at(114, 100), false, at(103, 100), true),
        (at(115, 100), false, at(104, 100), true),
        (
            MovementLocalPosition {
                floor: 6,
                ..at(104, 100)
            },
            true,
            at(103, 100),
            true,
        ),
    ]);
}

fn assert_fire_overlord_returns(
    cases: &[(MovementLocalPosition, bool, MovementLocalPosition, bool)],
) {
    use crate::ai_think::{
        behaviour_profile::CreatureBehaviourProfile, movement_goal::MovementGoal,
    };
    use crate::foundation::{
        CharacterId, CharacterLease, CommandId, CommandRef, OwnerDamageCommand,
    };

    const FIRE_OVERLORD: &str = "oteryn:creature.fire_overlord";
    let start = at(103, 100);
    let spawn = at(104, 100);
    for &(player_after, eligible, expected_boss_at, keep_damage) in cases {
        let mut channel = Channel::new();
        let boss = channel
            .runtime
            .admit_pinned_monster_lab_creature(start, FIRE_OVERLORD, 100)
            .expect("Fire Overlord actor");
        let profile = CreatureBehaviourProfile::project_native(Some(&rat(1)), &abilities())
            .expect("hostile native movement profile");
        let mut state = CreatureAiState::new(boss, profile, spawn);
        state.spawn = Some(spawn);
        channel.table.admit(state, 0);

        let character = CharacterId::decode(&uuid(9)).expect("character");
        let lease = CharacterLease::new(character, 1).expect("lease");
        channel
            .runtime
            .bind_attacker_lease(channel.player, channel.session, lease)
            .expect("bound attacker");
        channel.walk(at(114, 100), 0);
        channel
            .runtime
            .borrow_exact_actor_commit_at(0)
            .commit_damage_for_bound_attacker(
                boss,
                channel.player,
                CommandRef::new(channel.session, CommandId::new(1).expect("command")),
                0,
                OwnerDamageCommand {
                    target: FIRE_OVERLORD.as_bytes(),
                    occurrence: b"fire-overlord-walk-back",
                    binding: b"fire-overlord-walk-back",
                    damage: 10,
                },
            )
            .expect("physical player damage");
        channel.walk(at(115, 100), 0);
        let reports = channel.thinks(0, true);
        assert_eq!(reports[0].1.goal, MovementGoal::WalkBack { spawn });
        let due = channel
            .table
            .entry(boss)
            .expect("boss")
            .motion
            .due
            .expect("scheduled step");
        let roster = channel
            .runtime
            .all_source_player_damage_shares(boss)
            .expect("boss roster");
        assert_eq!(roster.len(), 1);
        assert_eq!((roster[0].actor, roster[0].total), (channel.player, 10));

        let mut player = channel.player_candidate();
        if player_after.floor != start.floor {
            let session = GameSessionId::decode(&uuid(12)).expect("viewer session");
            let reservation = channel
                .runtime
                .reserve_fresh_session(session)
                .expect("reserve viewer");
            player.actor = channel
                .runtime
                .commit_fresh_session(reservation)
                .expect("admit viewer");
            channel
                .runtime
                .initialize_pinned_test_position(player.actor, player_after)
                .expect("position viewer");
        } else if player_after != player.position {
            channel.walk(player_after, due - 1);
        }
        player.position = player_after;
        player.eligible = eligible;
        channel.advance_movement_with_players(due, true, &[player]);

        assert_eq!(channel.position(boss), expected_boss_at);
        let entry = channel.table.entry(boss).expect("boss");
        assert_eq!(entry.motion.goal, MovementGoal::Stand);
        assert_eq!(entry.motion.due, None);
        assert_eq!(
            channel
                .runtime
                .all_source_player_damage_shares(boss)
                .expect("boss roster"),
            roster,
            "movement keeps damage until a normal idle think"
        );
        assert_eq!(entry.due_us, Some(due));

        let reports = channel.table.run_due_thinks(
            &mut channel.runtime,
            due,
            &channel.root,
            &channel.revisions,
            &[player],
        );
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].1.idle, !keep_damage);
        assert_eq!(channel.table.due(boss).is_some(), keep_damage);
        assert_eq!(
            channel
                .runtime
                .all_source_player_damage_shares(boss)
                .expect("boss roster"),
            if keep_damage { roster } else { Vec::new() }
        );
    }
}
