//! AI-4 (GAME-AI-01 slice §4.6, §4.7): a creature bite issued by a typed AI issuer through Ability
//! and committed by the Channel owner. The vitals owner is a stand-in here with the same lethal
//! rule (DEATH-2 retired D54's floor); the real `ChannelSpellStates` path is tested beside it in
//! `actor_spell.rs`.
use super::super::exact_actor_test_ability::creature_bite::{
    AppliedBite, BiteRejection, CREATURE_BITE_LEDGER_MAX, CreatureBiteDefinition,
    CreatureBiteLedger, CreatureBiteVitals, CreatureDamage, CreatureHit, ReentryProtection,
    commit_ai_bite, creature_damage,
};
use super::super::exact_actor_test_ability::{AiAbilityAdapter, RevisionSet};
use super::super::owner_timer::SemanticTimeMicros;
use super::*;

fn uuid_v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ]
}

/// The player's vitals owner as the bite sees it: health, revision and the writes it took.
struct Vitals {
    health: u32,
    revision: u64,
    writes: usize,
}

impl CreatureBiteVitals for Vitals {
    fn apply_creature_damage(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _target_session: GameSessionId,
        magnitude: u32,
        _now: SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        // A dead player is no target.
        if self.health == 0 {
            return None;
        }
        let damage = creature_damage(self.health, magnitude);
        self.health = damage.health_after;
        self.revision += 1;
        self.writes += 1;
        Some(CreatureHit {
            damage,
            vitals_revision: self.revision,
            death: (damage.health_after == 0).then_some(uuid_v7(0x99)),
        })
    }
}

struct Owner {
    runtime: ChannelRuntimeV1,
    player: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
}

fn position(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

/// A runtime with one positioned player at (10, 10) and one live creature at `creature_at`.
fn owner(capacity: usize, creature_at: MovementLocalPosition) -> Owner {
    let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
        NodeId::decode(&uuid_v7(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        capacity,
        ChannelContentPin::test(world),
    )
    .expect("runtime");
    let session = GameSessionId::decode(&uuid_v7(0x21)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_movement_test_position(player, position(10, 10))
        .expect("player position");
    let creature = runtime.admit_test_creature(creature_at).expect("creature");
    Owner {
        runtime,
        player,
        session,
        creature,
    }
}

fn bite_definition() -> CreatureBiteDefinition {
    CreatureBiteDefinition::new(2_000_000, 8).expect("definition")
}

fn revisions() -> RevisionSet {
    RevisionSet::new(
        "ruleset:ai-v1",
        "content:1",
        "world:ai-v1",
        "formula:bite-v1",
        "simulation:v1",
    )
    .expect("revisions")
}

const UNPROTECTED: ReentryProtection = ReentryProtection {
    protected_until: None,
};

fn at(micros: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(micros)
}

#[allow(clippy::too_many_arguments)]
fn bite(
    owner: &Owner,
    ledger: &mut CreatureBiteLedger,
    vitals: &mut Vitals,
    issuer: ExactActorRef,
    sequence: u64,
    protection: ReentryProtection,
    now: u64,
) -> Result<AppliedBite, BiteRejection> {
    commit_ai_bite(
        ledger,
        &owner.runtime,
        vitals,
        AiAbilityAdapter::bite(issuer, sequence, owner.player, owner.session),
        bite_definition(),
        revisions(),
        protection,
        at(now),
    )
}

fn vitals(health: u32) -> Vitals {
    Vitals {
        health,
        revision: 1,
        writes: 0,
    }
}

#[test]
fn an_adjacent_bite_commits_once_and_a_retry_returns_the_first_result() {
    let owner = owner(4, position(11, 11));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    let first = bite(
        &owner,
        &mut ledger,
        &mut vitals,
        owner.creature,
        0,
        UNPROTECTED,
        0,
    );
    let expected = AppliedBite {
        requested: 8,
        damage: CreatureDamage {
            applied: 8,
            health_after: 177,
        },
        vitals_revision: 2,
        death: None,
    };
    assert_eq!(first, Ok(expected));
    for now in [0, 5_000_000] {
        assert_eq!(
            bite(
                &owner,
                &mut ledger,
                &mut vitals,
                owner.creature,
                0,
                UNPROTECTED,
                now
            ),
            Ok(expected)
        );
    }
    assert_eq!((vitals.health, vitals.writes), (177, 1));
}

#[test]
fn the_bite_interval_gates_the_next_think_occurrence() {
    let owner = owner(4, position(10, 11));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    assert!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            0,
            UNPROTECTED,
            0
        )
        .is_ok()
    );
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            1,
            UNPROTECTED,
            1_999_999
        ),
        Err(BiteRejection::CoolingDown)
    );
    assert!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            2,
            UNPROTECTED,
            2_000_000
        )
        .is_ok()
    );
    assert_eq!((vitals.health, vitals.writes), (169, 2));
    // An older occurrence cannot resolve after a newer one; another target is a conflict.
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            1,
            UNPROTECTED,
            9_000_000
        ),
        Err(BiteRejection::OccurrenceSuperseded)
    );
    assert_eq!(
        commit_ai_bite(
            &mut ledger,
            &owner.runtime,
            &mut vitals,
            AiAbilityAdapter::bite(owner.creature, 2, owner.creature, owner.session),
            bite_definition(),
            revisions(),
            UNPROTECTED,
            at(9_000_000),
        ),
        Err(BiteRejection::OccurrenceConflict)
    );
    assert_eq!(vitals.writes, 2);
}

#[test]
fn a_stale_or_non_creature_issuer_is_rejected_without_state() {
    let mut owner = owner(4, position(11, 10));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.player,
            0,
            UNPROTECTED,
            0
        ),
        Err(BiteRejection::StaleIssuer)
    );
    let creature = owner.creature;
    owner.runtime.remove_test_actor(creature).expect("despawn");
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            creature,
            0,
            UNPROTECTED,
            0
        ),
        Err(BiteRejection::StaleIssuer)
    );
    assert_eq!((vitals.health, vitals.writes, ledger.len()), (185, 0, 0));
}

#[test]
fn a_stale_target_or_a_distant_one_gets_no_bite() {
    let owner = owner(4, position(12, 10));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            0,
            UNPROTECTED,
            0
        ),
        Err(BiteRejection::OutOfRange)
    );
    let other = GameSessionId::decode(&uuid_v7(0x22)).expect("session");
    assert_eq!(
        commit_ai_bite(
            &mut ledger,
            &owner.runtime,
            &mut vitals,
            AiAbilityAdapter::bite(owner.creature, 1, owner.player, other),
            bite_definition(),
            revisions(),
            UNPROTECTED,
            at(0),
        ),
        Err(BiteRejection::StaleTarget)
    );
    assert_eq!((vitals.health, vitals.writes), (185, 0));
}

#[test]
fn a_protected_target_gets_no_bite_and_none_is_buffered() {
    let owner = owner(4, position(9, 9));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    let protected = ReentryProtection {
        protected_until: Some(at(4_000_000)),
    };
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            0,
            protected,
            3_999_999
        ),
        Err(BiteRejection::TargetProtected)
    );
    // The same occurrence after the window ends still returns its first result.
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            0,
            UNPROTECTED,
            4_000_000
        ),
        Err(BiteRejection::TargetProtected)
    );
    assert_eq!((vitals.health, vitals.writes), (185, 0));
    // A rejection starts no cooldown: the next think bites once the window is over.
    assert!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            1,
            protected,
            4_000_000
        )
        .is_ok()
    );
    assert_eq!(vitals.writes, 1);
}

/// DEATH-2 §4.1/§4.2: a hit to 0 is lethal and carries the minted death occurrence; the dead
/// player is no longer a target, and the lethal result is kept for its think occurrence.
#[test]
fn a_hit_to_zero_is_lethal_and_the_dead_player_is_no_target() {
    let owner = owner(4, position(11, 9));
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(3);
    let first = bite(
        &owner,
        &mut ledger,
        &mut vitals,
        owner.creature,
        0,
        UNPROTECTED,
        0,
    )
    .expect("first bite");
    assert_eq!(
        first.damage,
        CreatureDamage {
            applied: 3,
            health_after: 0
        }
    );
    assert_eq!(first.death, Some(uuid_v7(0x99)));
    // A retry of the lethal think occurrence returns the same result and writes nothing.
    assert_eq!(
        bite(
            &owner,
            &mut ledger,
            &mut vitals,
            owner.creature,
            0,
            UNPROTECTED,
            0,
        ),
        Ok(first)
    );
    let second = bite(
        &owner,
        &mut ledger,
        &mut vitals,
        owner.creature,
        1,
        UNPROTECTED,
        2_000_000,
    );
    assert_eq!(second, Err(BiteRejection::StaleTarget));
    assert_eq!((vitals.health, vitals.writes), (0, 1));
}

#[test]
fn the_ledger_holds_the_envelope_and_refuses_one_more_live_issuer() {
    // §4.9 derived bound: 16 spawn sources x 4 creatures.
    assert_eq!(
        CREATURE_BITE_LEDGER_MAX,
        AI01_SPAWN_SOURCES_PER_SCOPE_MAX * AI01_SPAWN_POPULATION_MAX
    );
    let capacity = CREATURE_BITE_LEDGER_MAX + 2;
    let mut owner = owner(capacity, position(11, 10));
    let mut creatures = vec![owner.creature];
    for index in 1..=CREATURE_BITE_LEDGER_MAX {
        let x = i32::try_from(index).expect("x") + 100;
        creatures.push(
            owner
                .runtime
                .admit_test_creature(position(x, 100))
                .expect("creature"),
        );
    }
    let mut ledger = CreatureBiteLedger::default();
    let mut vitals = vitals(185);
    for creature in &creatures[..CREATURE_BITE_LEDGER_MAX] {
        // Out of range, but each issuer's think still resolves and is kept.
        let _ = bite(
            &owner,
            &mut ledger,
            &mut vitals,
            *creature,
            0,
            UNPROTECTED,
            0,
        );
    }
    assert_eq!(ledger.len(), CREATURE_BITE_LEDGER_MAX);
    let extra = creatures[CREATURE_BITE_LEDGER_MAX];
    assert_eq!(
        bite(&owner, &mut ledger, &mut vitals, extra, 0, UNPROTECTED, 0),
        Err(BiteRejection::LedgerFull)
    );
    // A dead issuer's entry is dropped for a live one; retire drops it at once.
    let dead = creatures[1];
    owner.runtime.remove_test_actor(dead).expect("despawn");
    assert_eq!(
        bite(&owner, &mut ledger, &mut vitals, extra, 0, UNPROTECTED, 0),
        Err(BiteRejection::OutOfRange)
    );
    assert_eq!(ledger.len(), CREATURE_BITE_LEDGER_MAX);
    ledger.retire(extra);
    assert_eq!(ledger.len(), CREATURE_BITE_LEDGER_MAX - 1);
    assert_eq!(vitals.writes, 1);
}
