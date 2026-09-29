//! D132/D3-3: runtime per-attacker damage-contributor accumulation and its deterministic
//! top-damage tie-break.
//!
//! `docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
//! §4.3 (D132) and `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`'s
//! `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` row. This module is a child of
//! `runtime_actor_carrier` (`#[path]`-included), so it can reach the private
//! `DamageContributors`/`DamageContributor` types directly for white-box boundary and tie-break
//! coverage, alongside black-box coverage through the `CombatDeathFixture` seam D3-2 will later
//! compose against.

use super::*;

fn character(seed: u8) -> CharacterId {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    CharacterId::decode(&bytes).expect("uuid v7 fixture")
}

fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[8..].copy_from_slice(&raw.to_be_bytes());
        bytes[6] = 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        bytes
    }
    PreProductionContinuityGrant {
        world_id: WorldId::decode(&uuid_v7(seed)).expect("world"),
        channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("channel"),
        scope_generation: ScopeOwnershipGeneration::new(generation).expect("scope"),
    }
}

fn creature_fixture(seed: u64) -> CombatDeathFixture {
    let scope = grant(seed, 1);
    CombatDeathFixture::new(scope.world_id, scope.channel_id, scope.scope_generation)
        .expect("fixture creature")
}

// --- White-box: `DamageContributors` boundary and tie-break, independent of the carrier ---

#[test]
fn sixteen_distinct_contributors_are_tracked_and_a_seventeenth_is_not() {
    let mut contributors = DamageContributors::default();
    for seed in 1..=16_u8 {
        contributors.record(character(seed), 1);
    }
    assert_eq!(contributors.entries.len(), 16);
    // The 17th distinct contributor deals more damage than anyone tracked, but must never be
    // added to the map and must never become (or affect) the winner.
    contributors.record(character(17), 1_000);
    assert_eq!(
        contributors.entries.len(),
        COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
    );
    assert!(
        contributors
            .entries
            .iter()
            .all(|entry| entry.character != character(17))
    );
    // All 16 tracked contributors tie at total 1; the earliest (seed 1, ordinal 0) wins.
    assert_eq!(contributors.top_damage_character(), Some(character(1)));
}

#[test]
fn an_already_tracked_contributor_keeps_accumulating_past_the_cap() {
    let mut contributors = DamageContributors::default();
    for seed in 1..=16_u8 {
        contributors.record(character(seed), 1);
    }
    contributors.record(character(17), 1); // untracked: map already at capacity
    // character(1) is already tracked, so it keeps accumulating even though the map is full.
    contributors.record(character(1), 10);
    assert_eq!(contributors.entries.len(), 16);
    assert_eq!(contributors.top_damage_character(), Some(character(1)));
}

#[test]
fn equal_top_totals_resolve_to_the_contributor_that_reached_it_first() {
    let mut contributors = DamageContributors::default();
    contributors.record(character(1), 3); // ordinal 0: character(1) total = 3
    contributors.record(character(2), 5); // ordinal 1: character(2) total = 5 (current max)
    contributors.record(character(1), 2); // ordinal 2: character(1) total = 5 (tied)
    // Both now total 5; character(2) reached 5 at the earlier ordinal (1 < 2), so it wins even
    // though character(1) struck first chronologically.
    assert_eq!(contributors.top_damage_character(), Some(character(2)));
}

#[test]
fn a_same_ordinal_tie_resolves_to_the_lowest_character_id() {
    // Constructed directly: `record` always assigns a fresh, strictly increasing ordinal per
    // call, so two distinct contributors can never share one through the public accumulation
    // path alone. D132 rule 2 (simultaneous application within the same deterministic ordinal)
    // is still a defined, deterministic outcome, proven here directly against the data.
    let higher = DamageContributor {
        character: character(9),
        total: 7,
        last_update_ordinal: 4,
    };
    let lower = DamageContributor {
        character: character(3),
        total: 7,
        last_update_ordinal: 4,
    };
    let contributors = DamageContributors {
        entries: vec![higher, lower],
        next_ordinal: 5,
    };
    assert_eq!(contributors.top_damage_character(), Some(character(3)));
    // Order-independent: the reverse entry order resolves identically.
    let reversed = DamageContributors {
        entries: vec![lower, higher],
        next_ordinal: 5,
    };
    assert_eq!(reversed.top_damage_character(), Some(character(3)));
}

#[test]
fn top_damage_character_is_independent_of_entry_insertion_order() {
    let base = [
        DamageContributor {
            character: character(4),
            total: 12,
            last_update_ordinal: 6,
        },
        DamageContributor {
            character: character(1),
            total: 20,
            last_update_ordinal: 2,
        },
        DamageContributor {
            character: character(8),
            total: 20,
            last_update_ordinal: 5,
        },
        DamageContributor {
            character: character(2),
            total: 3,
            last_update_ordinal: 0,
        },
    ];
    // character(1) and character(8) tie at the highest total (20); character(1) reached it at
    // the earlier ordinal (2 < 5), so it is the one deterministic winner regardless of how the
    // four entries happen to be ordered in the backing Vec.
    let expected = Some(character(1));
    let permutations: [[usize; 4]; 6] = [
        [0, 1, 2, 3],
        [3, 2, 1, 0],
        [1, 0, 3, 2],
        [2, 3, 0, 1],
        [1, 2, 3, 0],
        [3, 0, 2, 1],
    ];
    for permutation in permutations {
        let entries = permutation.into_iter().map(|index| base[index]).collect();
        let shuffled = DamageContributors {
            entries,
            next_ordinal: 7,
        };
        assert_eq!(shuffled.top_damage_character(), expected);
    }
}

#[test]
fn no_contributors_yields_no_top_damage_character() {
    assert_eq!(DamageContributors::default().top_damage_character(), None);
}

// --- Black-box: the same behavior through the owner-committed carrier seam ---
//
// `CombatDeathFixture`/the carrier's `commit_creature_damage_inner` (VSL_COMBAT_FIXTURE_PROFILE,
// this file's own top-of-module doc comment: "Nonshipping fixed-one-creature Combat structural
// boundary") retains exactly one committed `OwnerCommitRecord` per creature generation -- a
// second, differently-keyed commit against the same live actor is an `OccurrenceConflict`, not a
// second applied hit. Multi-hit combat is unscoped future work, not something D3-3 introduces or
// can prove through today's fixture. The boundary/tie-break/overflow coverage above is therefore
// proven directly against `DamageContributors` (a pure function of already-recorded state,
// independent of how many real carrier hits ever produce that state); this seam test proves only
// that the one hit the fixture *does* support is correctly attributed to its attacker.

#[test]
fn carrier_seam_attributes_its_one_committed_hit_to_its_attacker() {
    let mut creature = creature_fixture(200);
    let result = creature
        .strike_by("strike:only", 7, character(1))
        .expect("the fixture's one committed hit");
    assert!(result.applied);
    assert_eq!(result.health_before - result.health_after, 7);
    assert_eq!(
        creature.top_damage_character().expect("lookup"),
        Some(character(1))
    );
}

#[test]
fn carrier_seam_never_attributes_a_replayed_occurrence_twice() {
    let mut creature = creature_fixture(201);
    let first = creature
        .strike_by("strike:replay", 4, character(1))
        .expect("first commit");
    assert!(first.applied);
    // An identical replay of the same occurrence/binding/damage returns the memoized,
    // not-applied transition (idempotency, `commit_creature_damage_inner`'s `prior` branch) --
    // it must never re-attribute a second contribution for the same attacker.
    let replay = creature
        .strike_by("strike:replay", 4, character(1))
        .expect("idempotent replay");
    assert!(!replay.applied);
    assert_eq!(
        creature.top_damage_character().expect("lookup"),
        Some(character(1))
    );
}

#[test]
fn accumulation_never_survives_removal_or_respawn() {
    let scope = grant(210, 1);
    let mut owner = NamespaceContinuityGuard::from_pre_production_grant(scope);
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1).expect("one slot");
    let first = carrier
        .admit_creature(&owner, ActorState(1), "fixture:d3-3.respawn", 20)
        .expect("first generation");
    carrier
        .current_owner_exact_commit(&owner)
        .commit_damage_for_attacker(
            ExactActorRef(first),
            character(1),
            OwnerDamageCommand {
                target: b"fixture:d3-3.respawn",
                occurrence: b"strike:before-removal",
                binding: b"strike:before-removal\0fixture:d3-3.respawn.v1",
                damage: 5,
            },
        )
        .expect("tracked hit");
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .top_damage_character(ExactActorRef(first))
            .expect("lookup before removal"),
        Some(character(1))
    );

    carrier
        .remove(&owner, first)
        .expect("administrative removal");
    let respawned = carrier
        .admit_creature(&owner, ActorState(2), "fixture:d3-3.respawn", 20)
        .expect("fresh generation");
    assert_ne!(
        first.actor_local_generation,
        respawned.actor_local_generation
    );
    // A fresh generation starts with no tracked contributors at all: not merely "different from
    // before" but genuinely empty, so nothing grows across generations.
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .top_damage_character(ExactActorRef(respawned))
            .expect("lookup after respawn"),
        None
    );
}
