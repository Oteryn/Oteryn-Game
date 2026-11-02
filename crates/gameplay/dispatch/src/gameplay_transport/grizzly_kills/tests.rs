#![allow(clippy::expect_used, reason = "closed identity fixtures must be valid")]

use super::*;
use crate::foundation::{
    ChannelId, CombatDeathFixture, PartyMemberSnapshot, PartyPresence, ScopeOwnershipGeneration,
    WorldId,
};

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn actor(seed: u8) -> ExactActorRef {
    CombatDeathFixture::new(
        WorldId::decode(&id(90)).expect("world"),
        ChannelId::decode(&id(seed)).expect("channel"),
        ScopeOwnershipGeneration::new(1).expect("scope"),
    )
    .expect("fixture")
    .actor()
}

fn snapshot(seed: u8, active: &[u8], revision: u64) -> TaskSnapshot {
    TaskSnapshot {
        actor: actor(seed),
        session: GameSessionId::decode(&id(seed)).expect("session"),
        character: id(seed),
        lease_generation: 2,
        observed_revision: revision,
        active_tasks: active.to_vec(),
    }
}

fn party(principal: u8, members: &[u8], shared: bool) -> PartySnapshot {
    let grouped = !members.is_empty();
    PartySnapshot {
        world: id(90),
        character: id(principal),
        party_id: grouped.then(|| id(80)),
        revision: grouped.then_some(1),
        leader: members.first().map(|member| id(*member)),
        shared_xp_enabled: shared,
        members: members
            .iter()
            .enumerate()
            .map(|(index, member)| PartyMemberSnapshot {
                character: id(*member),
                invitation_order: u64::try_from(index + 1).expect("order"),
                presence_revision: 1,
                presence: PartyPresence::Channel,
                channel: Some(id(90)),
            })
            .collect(),
        observed_database_micros: 1_000_000,
    }
}

fn credited(death: &FrozenGrizzlyDeath) -> Vec<[u8; 16]> {
    death
        .recipients
        .iter()
        .map(|recipient| recipient.character)
        .collect()
}

#[test]
fn grizzly_solo_and_inactive_shared_experience_credit_only_most_damage_principal() {
    let snapshots = [snapshot(1, &[1], 10), snapshot(2, &[1], 11)];
    for source in [party(1, &[], false), party(1, &[1, 2], false)] {
        let frozen = freeze(
            Some(source),
            id(1),
            true,
            "oteryn:creature.crocodile",
            &snapshots,
        );
        assert_eq!(credited(&frozen), vec![id(1)]);
        assert_eq!(
            frozen.recipients[0].actions,
            vec![GrizzlyAction::KillCreature {
                task_id: 1,
                creature: "oteryn:creature.crocodile".into(),
                observed_revision: 10,
            }]
        );
    }
}

#[test]
fn grizzly_shared_party_credits_each_matching_active_task_without_damage_or_range_filter() {
    let snapshots = [
        snapshot(1, &[9], 10),
        snapshot(2, &[1], 11),
        snapshot(3, &[9], 12),
    ];
    let frozen = freeze(
        Some(party(1, &[1, 2, 3], true)),
        id(1),
        true,
        "oteryn:creature.kongra",
        &snapshots,
    );
    assert_eq!(credited(&frozen), vec![id(1), id(3)]);
    assert_eq!(
        frozen.recipients[1].actions,
        vec![GrizzlyAction::KillCreature {
            task_id: 9,
            creature: "oteryn:creature.kongra".into(),
            observed_revision: 12,
        }]
    );
}

#[test]
fn grizzly_gone_most_damage_contributor_cannot_redirect_to_lethal_fallback() {
    let snapshots = [snapshot(1, &[1], 10), snapshot(2, &[1], 11)];
    let frozen = freeze(
        Some(party(2, &[1, 2], true)),
        id(2),
        false,
        "crocodile",
        &snapshots,
    );
    assert!(frozen.recipients.is_empty());
    assert!(frozen.source.is_none());
}

#[test]
fn grizzly_untracked_death_has_no_most_damage_player_and_cannot_credit_lethal_fallback() {
    let mut fixture = CombatDeathFixture::new(
        WorldId::decode(&id(90)).expect("world"),
        ChannelId::decode(&id(91)).expect("channel"),
        ScopeOwnershipGeneration::new(1).expect("scope"),
    )
    .expect("fixture");
    fixture
        .strike("unattributed lethal", CombatDeathFixture::HEALTH)
        .expect("death");
    let target = fixture.actor();
    let top = fixture
        .borrow_combat_death()
        .top_damage_contributor(target)
        .expect("top");
    assert!(
        !top.is_gone(),
        "the native untracked case is separate from a gone player"
    );
    assert!(top.present().is_none());
    let frozen = freeze(
        Some(party(1, &[], false)),
        id(1),
        top.present().is_some(),
        "crocodile",
        &[snapshot(1, &[1], 10)],
    );
    assert!(frozen.recipients.is_empty());
    assert!(frozen.source.is_none());
}

#[test]
fn grizzly_unknown_or_malformed_party_observation_never_invents_a_solo_award() {
    let snapshots = [snapshot(1, &[1], 10)];
    let mut malformed = party(1, &[1, 2], true);
    malformed.members[1].character = id(1);
    for source in [None, Some(malformed), Some(party(2, &[], false))] {
        assert!(
            freeze(source, id(1), true, "crocodile", &snapshots)
                .recipients
                .is_empty()
        );
    }
}

#[test]
fn grizzly_party_join_or_task_start_after_death_never_changes_delayed_or_replayed_credit() {
    let mut cache = GrizzlyCaptureCache {
        sessions: vec![
            snapshot(1, &[], 10),
            snapshot(2, &[1], 11),
            snapshot(3, &[1], 12),
        ],
        deaths: Vec::new(),
    };
    let original_party = party(1, &[1, 2], true);
    let original = cache.remember(actor(70), id(71), |snapshots| {
        freeze(
            Some(original_party.clone()),
            id(1),
            true,
            "crocodile",
            snapshots,
        )
    });
    assert_eq!(credited(&original), vec![id(2)]);
    // The principal starts its task and a third member joins before the death drains.
    cache.sessions[0] = snapshot(1, &[1], 20);
    let mut later_party = party(1, &[1, 2, 3], true);
    later_party.revision = Some(2);
    let replay = cache.remember(actor(70), id(71), |snapshots| {
        freeze(
            Some(later_party.clone()),
            id(1),
            true,
            "crocodile",
            snapshots,
        )
    });
    assert!(Arc::ptr_eq(&original, &replay));
    assert_eq!(credited(&replay), vec![id(2)]);
    assert_eq!(replay.source, Some(original_party));
    assert_eq!(cache.deaths.len(), 1);
    // A genuinely later death sees both changes, under a different occurrence.
    let next = cache.remember(actor(72), id(73), |snapshots| {
        freeze(Some(later_party), id(1), true, "crocodile", snapshots)
    });
    assert_eq!(credited(&next), vec![id(1), id(2), id(3)]);
}

#[test]
fn grizzly_death_preserves_original_run_revision_across_cancel_and_restart() {
    let mut cache = GrizzlyCaptureCache {
        sessions: vec![snapshot(1, &[1], 10)],
        deaths: Vec::new(),
    };
    let original = cache.remember(actor(70), id(71), |snapshots| {
        freeze(
            Some(party(1, &[], false)),
            id(1),
            true,
            "crocodile",
            snapshots,
        )
    });
    cache.sessions[0] = snapshot(1, &[1], 30);
    let replay = cache.remember(actor(70), id(71), |snapshots| {
        freeze(
            Some(party(1, &[], false)),
            id(1),
            true,
            "crocodile",
            snapshots,
        )
    });
    assert_eq!(
        replay.recipients[0].actions,
        vec![GrizzlyAction::KillCreature {
            task_id: 1,
            creature: "crocodile".into(),
            observed_revision: 10,
        }]
    );
    assert_eq!(replay, original);
}

#[test]
fn grizzly_boss_kill_does_not_require_joining_or_starting_a_task() {
    let snapshots = [snapshot(1, &[], 10), snapshot(2, &[], 11)];
    let frozen = freeze(
        Some(party(1, &[1, 2], true)),
        id(1),
        true,
        "the snapper",
        &snapshots,
    );
    assert_eq!(credited(&frozen), vec![id(1), id(2)]);
    for (recipient, observed_revision) in frozen.recipients.iter().zip([10, 11]) {
        assert_eq!(
            recipient.actions,
            vec![GrizzlyAction::BossKill {
                boss_id: 1,
                observed_revision
            }]
        );
    }
}

#[test]
fn grizzly_invalid_revision_or_excess_active_tasks_fail_closed() {
    let snapshots = [snapshot(1, &[1], 0), snapshot(2, &[1, 2, 3, 4], 11)];
    assert!(
        freeze(
            Some(party(1, &[1, 2], true)),
            id(1),
            true,
            "crocodile",
            &snapshots
        )
        .recipients
        .is_empty()
    );
}
