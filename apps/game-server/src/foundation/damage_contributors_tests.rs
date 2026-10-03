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

fn session(seed: u8) -> GameSessionId {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    GameSessionId::decode(&bytes).expect("uuid v7 session fixture")
}

/// `character(who)`'s command `sequence` (sub-occurrence `sub_ordinal`) of session `session_seed`.
fn attack(who: u8, session_seed: u8, sequence: u64, sub_ordinal: u16) -> AttackerCommand {
    // The lease generation is the session seed: a higher session seed is a newer session.
    AttackerCommand::new(
        character(who),
        u64::from(session_seed),
        CommandRef::new(
            session(session_seed),
            super::super::CommandId::new(sequence).expect("non-zero command"),
        ),
        sub_ordinal,
    )
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
        contributors.record(character(seed), 1, u64::from(seed) - 1, None);
    }
    assert_eq!(contributors.entries.len(), 16);
    // The 17th distinct contributor deals more damage than anyone tracked, but must never be
    // added to the map and must never become (or affect) the winner.
    contributors.record(character(17), 1_000, 16, None);
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
        contributors.record(character(seed), 1, u64::from(seed) - 1, None);
    }
    contributors.record(character(17), 1, 16, None); // untracked: map already at capacity
    // character(1) is already tracked, so it keeps accumulating even though the map is full.
    contributors.record(character(1), 10, 17, None);
    assert_eq!(contributors.entries.len(), 16);
    assert_eq!(contributors.top_damage_character(), Some(character(1)));
}

#[test]
fn equal_top_totals_resolve_to_the_contributor_that_reached_it_first() {
    let mut contributors = DamageContributors::default();
    contributors.record(character(1), 3, 0, None); // ordinal 0: character(1) total = 3
    contributors.record(character(2), 5, 1, None); // ordinal 1: character(2) total = 5 (current max)
    contributors.record(character(1), 2, 2, None); // ordinal 2: character(1) total = 5 (tied)
    // Both now total 5; character(2) reached 5 at the earlier ordinal (1 < 2), so it wins even
    // though character(1) struck first chronologically.
    assert_eq!(contributors.top_damage_character(), Some(character(2)));
}

#[test]
fn a_same_ordinal_tie_resolves_to_the_lowest_character_id() {
    // Constructed directly: the owner assigns a fresh, strictly increasing ordinal (D142) per
    // distinct commit, so two distinct contributors can never share one through the public
    // accumulation path alone. D132 rule 2 (simultaneous application within the same deterministic ordinal)
    // is still a defined, deterministic outcome, proven here directly against the data.
    let higher = DamageContributor {
        character: character(9),
        total: 7,
        last_update_ordinal: 4,
        high_water: None,
    };
    let lower = DamageContributor {
        character: character(3),
        total: 7,
        last_update_ordinal: 4,
        high_water: None,
    };
    let contributors = DamageContributors {
        entries: vec![higher, lower],
    };
    assert_eq!(contributors.top_damage_character(), Some(character(3)));
    // Order-independent: the reverse entry order resolves identically.
    let reversed = DamageContributors {
        entries: vec![lower, higher],
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
            high_water: None,
        },
        DamageContributor {
            character: character(1),
            total: 20,
            last_update_ordinal: 2,
            high_water: None,
        },
        DamageContributor {
            character: character(8),
            total: 20,
            last_update_ordinal: 5,
            high_water: None,
        },
        DamageContributor {
            character: character(2),
            total: 3,
            last_update_ordinal: 0,
            high_water: None,
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
        let shuffled = DamageContributors { entries };
        assert_eq!(shuffled.top_damage_character(), expected);
    }
}

#[test]
fn no_contributors_yields_no_top_damage_character() {
    assert_eq!(DamageContributors::default().top_damage_character(), None);
}

// --- Black-box: the same behavior through the owner-committed carrier seam ---
//
// `CombatDeathFixture`/the carrier's `commit_creature_damage_inner` retain up to 16 committed
// receipts per creature generation (D4/D140), each assigned the owner damage-application ordinal
// (D142) that also feeds this accumulator. The fixture's `strike_by` stands in for the future
// transport: one monotonic command per distinct occurrence text.

#[test]
fn carrier_seam_attributes_a_committed_hit_to_its_attacker() {
    let mut creature = creature_fixture(200);
    let result = creature
        .strike_by("strike:only", 7, character(1))
        .expect("the fixture's committed hit");
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
    // not-applied transition (idempotency, `commit_creature_damage_inner`'s retained-receipt match) --
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
            attack(1, 1, 1, 0),
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

// --- D4/D142: the owner damage-application ordinal feeds this accumulator, end to end ---------

fn strike_as(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    actor: ActorRef,
    attacker: Option<AttackerCommand>,
    tag: &str,
    damage: i64,
) -> Result<OwnerDamageResult, CarrierError> {
    let binding = format!("{tag}\0fixture:d4.ordinal.v1").into_bytes();
    carrier.commit_creature_damage_inner(
        owner,
        actor,
        OwnerDamageCommand {
            target: b"fixture:d4.ordinal",
            occurrence: tag.as_bytes(),
            binding: &binding,
            damage,
        },
        attacker,
        false,
    )
}

fn ordinal_fixture(
    seed: u64,
    health: i64,
) -> (NamespaceContinuityGuard, ChannelActorCarrier, ActorRef) {
    let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1).expect("one slot");
    let actor = carrier
        .admit_creature(&owner, ActorState(1), "fixture:d4.ordinal", health)
        .expect("creature");
    (owner, carrier, actor)
}

fn slot_state(carrier: &ChannelActorCarrier) -> (&DamageReceipts, &DamageContributors) {
    match &carrier.slots[0] {
        Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } => Some((&***committed, &**damage_contributors)),
        _ => None,
    }
    .expect("expected the creature slot")
}

fn top(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    actor: ActorRef,
) -> Option<CharacterId> {
    carrier
        .current_owner_combat_death(owner)
        .top_damage_character(ExactActorRef(actor))
        .expect("lookup")
}

#[test]
fn the_owner_ordinal_is_gap_free_across_attackers_and_is_the_tie_break_input() {
    let (owner, mut carrier, actor) = ordinal_fixture(300, 1_000);
    // Known interleaved order across two attackers and one unattributed source.
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 1, 0)),
        "hit:a",
        3,
    )
    .expect("ord 0");
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(2, 1, 1, 0)),
        "hit:b",
        5,
    )
    .expect("ord 1");
    strike_as(&mut carrier, &owner, actor, None, "hit:env", 1).expect("ord 2");
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 2, 0)),
        "hit:c",
        2,
    )
    .expect("ord 3");

    let (receipts, contributors) = slot_state(&carrier);
    let ordinals: Vec<u64> = receipts
        .entries
        .iter()
        .map(|record| record.ordinal)
        .collect();
    assert_eq!(
        ordinals,
        vec![0, 1, 2, 3],
        "strictly increasing and gap-free across attackers"
    );
    assert_eq!(receipts.next_ordinal, 4);
    // The accumulator consumed the same sequence: the unattributed hit still advanced it, so
    // character(1)'s last update is ordinal 3 (a private attributed-only counter would say 2).
    let last_update = |seed: u8| {
        contributors
            .entries
            .iter()
            .find(|entry| entry.character == character(seed))
            .map(|entry| (entry.total, entry.last_update_ordinal))
    };
    assert_eq!(last_update(1), Some((5, 3)));
    assert_eq!(last_update(2), Some((5, 1)));
    // Equal top totals (5 and 5): character(2) reached 5 first (ordinal 1 < 3).
    assert_eq!(top(&mut carrier, &owner, actor), Some(character(2)));

    // A replay never consumes an ordinal.
    let before = carrier.slots.clone();
    assert!(
        !strike_as(
            &mut carrier,
            &owner,
            actor,
            Some(attack(1, 1, 1, 0)),
            "hit:a",
            3
        )
        .expect("replay")
        .applied
    );
    assert_eq!(carrier.slots, before);
    assert_eq!(slot_state(&carrier).0.next_ordinal, 4);

    // One more point breaks the tie outright.
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 3, 0)),
        "hit:d",
        1,
    )
    .expect("ord 4");
    assert_eq!(slot_state(&carrier).0.next_ordinal, 5);
    assert_eq!(top(&mut carrier, &owner, actor), Some(character(1)));
}

#[test]
fn overkill_credits_only_removed_hp_and_the_tie_break_uses_the_shared_ordinal() {
    // 12 HP: character(2) 6 at ordinal 0; character(1) 5 at ordinal 1; character(1)'s overkill
    // (100 requested) removes the last 1 HP at ordinal 2 -> both total 6; character(2) reached 6
    // first (ordinal 0 < 2), so character(2) wins.
    let (owner, mut carrier, actor) = ordinal_fixture(310, 12);
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(2, 1, 1, 0)),
        "hit:a",
        6,
    )
    .expect("6");
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 1, 0)),
        "hit:b",
        5,
    )
    .expect("5");
    let lethal = strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 2, 0)),
        "hit:c",
        100,
    )
    .expect("overkill");
    assert_eq!((lethal.health_before, lethal.health_after), (1, 0));
    let (receipts, contributors) = slot_state(&carrier);
    assert_eq!(receipts.lethal().map(|record| record.ordinal), Some(2));
    let total = |seed: u8| {
        contributors
            .entries
            .iter()
            .find(|entry| entry.character == character(seed))
            .map(|entry| entry.total)
    };
    assert_eq!(total(1), Some(6));
    assert_eq!(total(2), Some(6));
    assert_eq!(top(&mut carrier, &owner, actor), Some(character(2)));

    // 10 HP: an overkill after 3 removes only the remaining 1 HP and never overtakes a prior 6.
    let (owner, mut carrier, actor) = ordinal_fixture(320, 10);
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(2, 1, 1, 0)),
        "hit:a",
        6,
    )
    .expect("6");
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 1, 0)),
        "hit:b",
        3,
    )
    .expect("3");
    strike_as(
        &mut carrier,
        &owner,
        actor,
        Some(attack(1, 1, 2, 0)),
        "hit:c",
        100,
    )
    .expect("kill");
    let (_, contributors) = slot_state(&carrier);
    assert_eq!(
        contributors
            .entries
            .iter()
            .find(|entry| entry.character == character(1))
            .map(|entry| entry.total),
        Some(4)
    );
    assert_eq!(top(&mut carrier, &owner, actor), Some(character(2)));
}

#[test]
fn eviction_never_resets_the_ordinal_or_the_attackers_high_water_mark() {
    let (owner, mut carrier, actor) = ordinal_fixture(330, 1_000);
    for sequence in 1..=20_u64 {
        strike_as(
            &mut carrier,
            &owner,
            actor,
            Some(attack(1, 1, sequence, 0)),
            &format!("hit:{sequence}"),
            1,
        )
        .expect("hit");
    }
    let (receipts, contributors) = slot_state(&carrier);
    let ordinals: Vec<u64> = receipts
        .entries
        .iter()
        .map(|record| record.ordinal)
        .collect();
    assert_eq!(ordinals, (4..20).collect::<Vec<u64>>());
    assert_eq!(receipts.next_ordinal, 20);
    assert_eq!(
        contributors.high_water(character(1)),
        Some((1, session(1), 20, 0))
    );
    assert_eq!(top(&mut carrier, &owner, actor), Some(character(1)));
}
