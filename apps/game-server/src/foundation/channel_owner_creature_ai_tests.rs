#![allow(
    clippy::expect_used,
    reason = "test fixtures and assertions must fail on missing evidence"
)]

//! CREATURE-AI-1 §2.1 D116 on the Channel owner: two rats idle until a player walks into view,
//! then chase, swing their melee every 2000 ms when adjacent, flee when hurt, and go idle again
//! when the player leaves. Profile refusals admit nothing; a dead creature leaves the table.

use super::*;
use crate::ability::creature_bite::{
    CreatureBiteVitals, CreatureHit, ReentryProtection, creature_damage,
};
use crate::ai_monster_melee::{Dispatch, MeleeDefinition, MonsterMeleeOwner};
use crate::ai_think::{
    AttackReadiness, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId, StepKind,
};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelContentPin, ChannelId, GameSessionId, NodeId, WorldId};
use oteryn_simulation_determinism::DecisionOccurrenceId;
use serde_json::{Value, json};

const SECOND: u64 = 1_000_000;

struct Vitals {
    health: u32,
    revision: u64,
}

impl CreatureBiteVitals for Vitals {
    fn apply_creature_damage(
        &mut self,
        _: &mut ChannelRuntimeV1,
        _: ExactActorRef,
        _: GameSessionId,
        magnitude: u32,
        _: SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        let damage = creature_damage(self.health, magnitude);
        self.health = damage.health_after;
        self.revision += 1;
        Some(CreatureHit {
            damage,
            vitals_revision: self.revision,
            death: None,
        })
    }
}

fn uuid(tag: u8) -> [u8; 16] {
    [1, 144, 0, 0, 0, tag, 112, 0, 128, 0, 0, 0, 0, 0, 0, tag]
}

fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

fn reference(key: &str) -> Value {
    json!({"family":"Ability","key":format!("oteryn:ability.{key}"),"revision":"definition-r1"})
}

fn abilities() -> BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring> {
    [(
        serde_json::from_value(reference("bite")).expect("reference"),
        serde_json::from_value(json!({"details":{"kind":"Melee","range_tiles":1,"needs_target":true,"needs_direction":false}}))
            .expect("ability profile"),
    )]
    .into_iter()
    .collect()
}

/// The D116 rat: one melee entry, 2000 ms, {0, 8}; flees at 5 hp.
fn rat(melee_entries: usize) -> ProjectV2BehaviorAuthoring {
    let bite = json!({"ability":reference("bite"),"interval_ms":2000,"chance_ppm":1_000_000,"magnitude":{"minimum":0,"maximum":8}});
    serde_json::from_value(json!({
        "movement":{"can_walk":true,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false},
        "targeting":{"hostile":true,"can_target":true,"sense_invisible":false,"target_distance_tiles":1,
            "static_attack_chance_ppm":900_000,"flee_health":5,
            "change_target":{"interval_ms":4000,"chance_ppm":0},
            "strategy_weights":{"nearest":100,"damage":0,"health":0,"random":0}},
        "attacks":vec![bite; melee_entries],
        "defenses":[]}))
    .expect("behaviour fixture")
}

struct Channel {
    runtime: ChannelRuntimeV1,
    table: CreatureAiTable,
    player: ExactActorRef,
    session: GameSessionId,
    root: GameplayDecisionRoot,
    revisions: RevisionSet,
}

impl Channel {
    fn new() -> Self {
        Self::with_capacity(8)
    }

    fn with_capacity(capacity: usize) -> Self {
        let world = WorldId::decode(&uuid(1)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).expect("channel"),
            NodeId::decode(&uuid(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
            ChannelContentPin::test(world),
        )
        .expect("runtime");
        let session = GameSessionId::decode(&uuid(4)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let player = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .initialize_pinned_test_position(player, at(115, 100))
            .expect("player position");
        Self {
            runtime,
            table: CreatureAiTable::default(),
            player,
            session,
            root: GameplayDecisionRoot::from_bytes([9; 32]),
            revisions: RevisionSet::new(
                "rules-r1",
                "content-r1",
                "world-r1",
                "formula-r1",
                "sim-r1",
            )
            .expect("revisions"),
        }
    }

    fn admit(&mut self, position: MovementLocalPosition, now_us: u64) -> ExactActorRef {
        let creature = self
            .runtime
            .admit_pinned_test_creature(position)
            .expect("creature");
        self.table
            .admit_creature(&self.runtime, creature, Some(&rat(1)), &abilities(), now_us)
            .expect("admitted");
        creature
    }

    fn position(&self, actor: ExactActorRef) -> MovementLocalPosition {
        self.runtime
            .read_actor_position(actor)
            .expect("position")
            .position()
    }

    /// The player steps one tile; the owner wakes the idle creatures it now sees.
    fn walk(&mut self, to: MovementLocalPosition, now_us: u64) -> Vec<ExactActorRef> {
        let snapshot = self
            .runtime
            .read_actor_position(self.player)
            .expect("player");
        self.runtime
            .borrow_movement_position()
            .commit_cardinal(snapshot, to)
            .expect("player step");
        self.table
            .wake_for_player(&self.runtime, self.player, now_us)
            .expect("wake")
    }

    fn thinks(
        &mut self,
        now_us: u64,
        walkable: bool,
    ) -> Vec<(ExactActorRef, crate::ai_think::ThinkReport)> {
        let players = [TargetCandidate {
            actor: self.player,
            position: self.position(self.player),
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        }];
        self.table.run_due_thinks(
            &mut self.runtime,
            now_us,
            &self.root,
            &self.revisions,
            &players,
            |_| walkable,
        )
    }

    /// The profile's melee swing for `creature`, owned by the melee owner.
    fn swing(
        &mut self,
        melee: &mut MonsterMeleeOwner,
        vitals: &mut Vitals,
        creature: ExactActorRef,
        sequence: u64,
        now_us: u64,
    ) -> Dispatch {
        let player = PerceivedPlayer {
            id: PerceivedPlayerId::new(1),
            position: self.position(self.player),
            legal_attack_target: true,
        };
        self.swing_at(
            melee,
            vitals,
            creature,
            sequence,
            now_us,
            &[(self.player, self.session, player)],
        )
        .0
    }

    /// As `swing`, at the `perceived` players the production owner passes; with the player the
    /// swing resolved.
    fn swing_at(
        &mut self,
        melee: &mut MonsterMeleeOwner,
        vitals: &mut Vitals,
        creature: ExactActorRef,
        sequence: u64,
        now_us: u64,
        perceived: &[(ExactActorRef, GameSessionId, PerceivedPlayer)],
    ) -> (Dispatch, Option<ExactActorRef>) {
        let profile_melee = self
            .table
            .state(creature)
            .and_then(|state| state.profile().melee())
            .expect("melee entry");
        let definition = MeleeDefinition::new(
            self.runtime.content_pin().server_artifact_digest(),
            profile_melee.interval_ms,
            profile_melee.chance_ppm,
            u32::try_from(profile_melee.minimum).expect("minimum"),
            u32::try_from(profile_melee.maximum).expect("maximum"),
        )
        .expect("melee definition")
        .with_entry_index(profile_melee.index);
        let position = self.position(creature);
        let input = CreatureThinkInput {
            provenance: crate::ai::AiProvenance::new(crate::ai::AiProvenanceInput {
                scope_id: 1,
                scope_generation: 1,
                actor_generation: 1,
                behavior_revision: 1,
                content_revision: 1,
                navigation_revision: 1,
                ruleset_revision: 1,
                determinism_profile_revision: 1,
            }),
            decision_root: self.root.clone(),
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
            path_work_id: 1,
            position,
            home: position,
            attack: AttackReadiness {
                off_cooldown: false,
                chance_percent: 0,
            },
        };
        let players = perceived
            .iter()
            .map(|(_, _, player)| *player)
            .collect::<Vec<_>>();
        let resolved = std::cell::Cell::new(None);
        let dispatch = melee
            .think(
                &mut self.runtime,
                vitals,
                creature,
                sequence,
                definition,
                input,
                &players,
                |id| {
                    let (target, session, _) =
                        perceived.iter().find(|(_, _, player)| player.id == id)?;
                    resolved.set(Some(*target));
                    Some((
                        *target,
                        *session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                self.revisions.clone(),
                SemanticTimeMicros::from_micros(now_us),
            )
            .expect("swing");
        (dispatch, resolved.get())
    }
}

fn swung(dispatch: Dispatch) -> bool {
    matches!(dispatch, Dispatch::Bite(Ok(_)) | Dispatch::ZeroDamage)
}

#[test]
fn d116_rats_wake_chase_bite_flee_and_idle_on_the_channel_owner() {
    let mut channel = Channel::new();
    let a = channel.admit(at(100, 100), 0);
    let b = channel.admit(at(99, 104), 0);
    // The player at (115, 100) is out of both rats' view: the first thinks are idle.
    let reports = channel.thinks(0, true);
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|(_, report)| report.idle));
    assert_eq!(channel.table.pending(), 0);
    assert!(channel.thinks(SECOND, true).is_empty());

    // The player walks west; each rat wakes when the player first sees it: A at x = 108,
    // B (one tile further west) at x = 107.
    let mut now = 2 * SECOND;
    for x in (109..=114).rev() {
        assert!(channel.walk(at(x, 100), now).is_empty());
        now += 100_000;
    }
    assert_eq!(channel.walk(at(108, 100), now), vec![a]);
    now += 100_000;
    assert_eq!(channel.walk(at(107, 100), now), vec![b]);

    // They think every second, target the player and chase until adjacent.
    let mut melee = MonsterMeleeOwner::default();
    let mut vitals = Vitals {
        health: 1_000,
        revision: 0,
    };
    let mut bites = Vec::new();
    for think in 0..12 {
        let reports = channel.thinks(now, true);
        assert_eq!(reports.len(), 2, "think {think}");
        for (_, report) in &reports {
            assert_eq!(report.target, Some(channel.player));
            assert!(!report.idle && !report.fleeing);
            if let Some(step) = report.step {
                assert_eq!(step.kind, StepKind::Chase);
            }
        }
        let a_report = &reports.iter().find(|(actor, _)| *actor == a).expect("a").1;
        if swung(channel.swing(&mut melee, &mut vitals, a, a_report.sequence, now)) {
            bites.push(now);
        }
        now += SECOND;
    }
    let player = channel.position(channel.player);
    let a_at = channel.position(a);
    assert!((a_at.x - player.x).abs().max((a_at.y - player.y).abs()) <= 1);
    assert!(bites.len() >= 3);
    assert!(bites.windows(2).all(|pair| pair[1] - pair[0] == 2 * SECOND));

    // A hurt rat flees but keeps its target; a refused step still bites when adjacent.
    channel
        .runtime
        .set_test_creature_health(a, 5)
        .expect("hurt");
    let reports = channel.thinks(now, false);
    let a_report = &reports.iter().find(|(actor, _)| *actor == a).expect("a").1;
    assert!(a_report.fleeing);
    assert_eq!(a_report.target, Some(channel.player));
    assert_eq!(a_report.step, None);
    let last_bite = *bites.last().expect("bite");
    let sequence = a_report.sequence;
    let dispatch = channel.swing(&mut melee, &mut vitals, a, sequence, now);
    assert_eq!(swung(dispatch), now - last_bite >= 2 * SECOND);
    now += SECOND;
    let reports = channel.thinks(now, true);
    let a_report = &reports.iter().find(|(actor, _)| *actor == a).expect("a").1;
    assert!(a_report.fleeing);
    assert_eq!(a_report.step.map(|step| step.kind), Some(StepKind::Flee));
    now += SECOND;

    // The player walks east out of view; the rats go idle and stop thinking.
    for x in player.x + 1..=player.x + 20 {
        channel.walk(at(x, player.y), now);
    }
    loop {
        let reports = channel.thinks(now, true);
        now += SECOND;
        if reports.iter().all(|(_, report)| report.idle) {
            break;
        }
        assert!(now < 120 * SECOND, "rats never went idle");
    }
    assert_eq!(channel.table.pending(), 0);
    assert!(channel.thinks(now, true).is_empty());

    // A rat admitted in view of the player targets it on its first think.
    let player = channel.position(channel.player);
    let c = channel.admit(at(player.x - 3, player.y), now);
    let reports = channel.thinks(now, true);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].0, c);
    assert_eq!(reports[0].1.target, Some(channel.player));

    // Missing and invalid profiles admit nothing.
    let refused = channel
        .runtime
        .admit_pinned_test_creature(at(player.x - 4, player.y + 2))
        .expect("creature");
    assert_eq!(
        channel
            .table
            .admit_creature(&channel.runtime, refused, None, &abilities(), now),
        Err(CarrierError::ProfileMissing)
    );
    assert_eq!(
        channel
            .table
            .admit_creature(&channel.runtime, refused, Some(&rat(2)), &abilities(), now),
        Err(CarrierError::ProfileInvalid)
    );
    assert!(channel.table.state(refused).is_none());

    // A dead creature leaves the table at its next think.
    channel
        .runtime
        .set_test_creature_health(c, 0)
        .expect("dead");
    let before = channel.table.len();
    now += SECOND;
    assert!(
        channel
            .thinks(now, true)
            .iter()
            .all(|(actor, _)| *actor != c)
    );
    assert!(channel.table.state(c).is_none());
    assert_eq!(channel.table.len(), before - 1);
}

#[test]
fn the_swing_goes_at_the_think_target_and_not_a_nearer_player() {
    let mut channel = Channel::new();
    // The first player walks next to where the rat stands.
    for x in (101..=114).rev() {
        channel.walk(at(x, 100), 0);
    }
    let rat = channel.admit(at(100, 100), 0);
    let first = channel.player;
    let reports = channel.thinks(0, false);
    assert_eq!(reports.len(), 1);
    assert_eq!(
        channel.table.state(rat).and_then(|s| s.target()),
        Some(first)
    );

    // A second player arrives on the rat's other side, as near and listed first.
    let session = GameSessionId::decode(&uuid(5)).expect("session");
    let reservation = channel
        .runtime
        .reserve_fresh_session(session)
        .expect("reserve");
    let second = channel
        .runtime
        .commit_fresh_session(reservation)
        .expect("commit");
    channel
        .runtime
        .initialize_pinned_test_position(second, at(99, 100))
        .expect("second position");
    let candidate = |actor| TargetCandidate {
        actor,
        position: channel.position(actor),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    };
    let players = [candidate(second), candidate(first)];
    let reports = channel.table.run_due_thinks(
        &mut channel.runtime,
        SECOND,
        &channel.root,
        &channel.revisions,
        &players,
        |_| false,
    );
    assert_eq!(reports.len(), 1);
    let target = channel.table.state(rat).and_then(|s| s.target());
    assert_eq!(target, Some(first));

    // The production owner passes the swing only the think's target, so the nearest-first
    // swing cannot pick the second player.
    let perceived =
        [(second, session, 1), (first, channel.session, 2)].map(|(actor, session, id)| {
            (
                actor,
                session,
                PerceivedPlayer {
                    id: PerceivedPlayerId::new(id),
                    position: channel.position(actor),
                    legal_attack_target: true,
                },
            )
        });
    let passed = crate::ai_monster_melee::think_target_only(
        target,
        perceived.iter().map(|(actor, _, player)| (*actor, *player)),
    );
    assert_eq!(passed.len(), 1);
    assert_eq!(passed[0].id, PerceivedPlayerId::new(2));
    let mut melee = MonsterMeleeOwner::default();
    let mut vitals = Vitals {
        health: 1_000,
        revision: 0,
    };
    let only_target = perceived
        .iter()
        .filter(|(_, _, player)| passed.contains(player))
        .copied()
        .collect::<Vec<_>>();
    // Swung at every perceived player, the nearest-first swing would resolve the second.
    let mut first_swing = |melee: &mut MonsterMeleeOwner, vitals: &mut Vitals, perceived: &[_]| {
        (1..=8).find_map(|sequence| {
            let (dispatch, resolved) = channel.swing_at(
                melee,
                vitals,
                rat,
                sequence,
                SECOND + sequence * 2 * SECOND,
                perceived,
            );
            swung(dispatch).then_some(resolved).flatten()
        })
    };
    assert_eq!(
        first_swing(&mut MonsterMeleeOwner::default(), &mut vitals, &perceived),
        Some(second)
    );
    assert_eq!(
        first_swing(&mut melee, &mut vitals, &only_target),
        Some(first)
    );
    // No think target leaves the swing idle.
    assert!(
        crate::ai_monster_melee::think_target_only(
            None,
            perceived.iter().map(|(actor, _, player)| (*actor, *player)),
        )
        .is_empty()
    );
}

/// SPAWN-CEILINGS-1 (Codex 4242381774): 300 rats, past the old `AI01-ACTIVE-ACTORS` 256. The
/// melee census lists every one in slot order, also after a slot is reused, all 300 think in one
/// pass, and a rat that dies while the table holds it is forgotten without a think.
#[test]
fn three_hundred_rats_are_censused_in_slot_order_think_in_one_pass_and_the_dead_are_forgotten() {
    const RATS: usize = 300;
    let mut channel = Channel::with_capacity(RATS + 1);
    let rats = (0..RATS)
        .map(|index| {
            let x = 200 + 2 * i32::try_from(index).expect("x");
            channel.admit(at(x, 300), 0)
        })
        .collect::<Vec<_>>();
    let in_slot_order = |runtime: &ChannelRuntimeV1| {
        let census = runtime.positioned_melee_census().expect("census");
        assert_eq!(census.len(), RATS + 1);
        census
            .windows(2)
            .all(|pair| pair[0].0.actor_local_id() < pair[1].0.actor_local_id())
    };
    assert!(in_slot_order(&channel.runtime));
    assert_eq!(channel.table.len(), RATS);
    assert_eq!(channel.thinks(0, false).len(), RATS);

    let dead = rats[7];
    assert!(channel.runtime.kill_creature_for_test(dead));
    channel.table.forget_dead(&channel.runtime);
    assert_eq!(channel.table.len(), RATS - 1);
    assert!(channel.table.state(dead).is_none());
    assert!(channel.table.state(rats[8]).is_some());

    // The freed slot, the only one, takes the next rat; the census keeps slot order.
    channel.runtime.remove_test_actor(dead).expect("despawn");
    let newer = channel.admit(at(199, 300), 0);
    assert_eq!(newer.actor_local_id(), dead.actor_local_id());
    assert!(in_slot_order(&channel.runtime));
    assert_eq!(channel.table.len(), RATS);
}

/// SPAWN-CEILINGS-1 (Codex 4243189401): 1,500 rats due at once think 1,024 in the first owner
/// window and the rest in the next; `earliest_due` reports the left-over thinks as still due, so
/// the owner schedules its next pass in the next window.
#[test]
fn thinks_past_the_window_budget_stay_due_and_run_in_the_next_window() {
    use crate::ai_think::{CREATUREAI0_THINK_WINDOW_MICROS, CREATUREAI0_THINKS_PER_WINDOW};
    const RATS: usize = 1_500;
    let mut channel = Channel::with_capacity(RATS + 1);
    for index in 0..RATS {
        let x = 200 + 2 * i32::try_from(index % 100).expect("x");
        let y = 300 + 2 * i32::try_from(index / 100).expect("y");
        channel.admit(at(x, y), 0);
    }
    assert_eq!(channel.table.earliest_due(), Some(0));
    assert_eq!(
        channel.thinks(0, false).len(),
        CREATUREAI0_THINKS_PER_WINDOW
    );
    assert_eq!(channel.table.earliest_due(), Some(0));
    assert!(
        channel
            .thinks(CREATUREAI0_THINK_WINDOW_MICROS - 1, false)
            .is_empty()
    );
    assert_eq!(
        channel.thinks(CREATUREAI0_THINK_WINDOW_MICROS, false).len(),
        RATS - CREATUREAI0_THINKS_PER_WINDOW
    );
    assert!(
        channel
            .table
            .earliest_due()
            .is_none_or(|due| due >= 1_000_000)
    );
}

/// SPAWN-CEILINGS-1 (Codex 4243397472): the owner's bounded selection takes exactly the thinks
/// `take_due`'s full sort takes, in the same order and against the same window budget, and
/// reports the earliest think it left; after a pass, the reported next think is the table's
/// `earliest_due`, so the owner needs no second walk of the table.
#[test]
fn the_bounded_think_selection_equals_the_full_sort_and_reports_the_next_think() {
    use crate::ai_think::{CREATUREAI0_THINK_WINDOW_MICROS, CREATUREAI0_THINKS_PER_WINDOW};
    const RATS: usize = 1_500;
    let due_of = |index: usize| u64::try_from((index * 7) % 3).expect("small") * 20_000;
    let channel_of = || {
        let mut channel = Channel::with_capacity(RATS + 1);
        for index in 0..RATS {
            let x = 200 + 2 * i32::try_from(index % 100).expect("x");
            let y = 300 + 2 * i32::try_from(index / 100).expect("y");
            channel.admit(at(x, y), due_of(index));
        }
        channel
    };
    let mut sorted = channel_of();
    let mut bounded = channel_of();
    for now in [
        45_000,
        45_001,
        CREATUREAI0_THINK_WINDOW_MICROS,
        CREATUREAI0_THINK_WINDOW_MICROS + 30_000,
    ] {
        let expected = sorted.table.take_due(now);
        let (taken, rest) = bounded.table.take_due_bounded(now);
        assert_eq!(taken, expected, "selection at {now}");
        let left = (0..RATS)
            .map(due_of)
            .filter(|due| *due <= now)
            .count()
            .saturating_sub(taken.len());
        if now == 45_000 {
            assert_eq!(taken.len(), CREATUREAI0_THINKS_PER_WINDOW);
            assert_eq!(rest, Some(40_000), "the earliest think left at {now}");
        } else {
            assert!(left == 0 || rest.is_some(), "{left} thinks left unreported");
        }
    }

    let mut channel = channel_of();
    let players = [TargetCandidate {
        actor: channel.player,
        position: channel.position(channel.player),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    for now in [0, 20_000, 45_000, CREATUREAI0_THINK_WINDOW_MICROS] {
        let (_, next) = channel.table.run_due_thinks_until_next(
            &mut channel.runtime,
            now,
            &channel.root,
            &channel.revisions,
            &players,
            |_| false,
        );
        assert_eq!(next, channel.table.earliest_due(), "next think after {now}");
    }
}

/// SPAWN-CEILINGS-1 (Codex 4242808144): the wake's view box holds every cell a player sees, on
/// every floor pair, VIS-1's 0..=15 and the source-world floors outside it.
#[test]
fn the_wake_view_box_holds_every_cell_a_player_sees() {
    for player_floor in -2..=17 {
        let player = MovementLocalPosition {
            x: 1_000,
            y: 1_000,
            floor: player_floor,
        };
        for floor in -2..=17 {
            let wake_box = wake_view_box(player, floor);
            if let Some((columns, rows)) = &wake_box {
                assert_eq!((columns.clone().count(), rows.clone().count()), (19, 19));
            }
            for dx in -30..=30 {
                for dy in -30..=30 {
                    let creature = MovementLocalPosition {
                        x: player.x + dx,
                        y: player.y + dy,
                        floor,
                    };
                    if targeting::sees(player, creature) {
                        let (columns, rows) = wake_box.clone().expect("seen floor in the box");
                        assert!(columns.contains(&creature.x) && rows.contains(&creature.y));
                    }
                }
            }
        }
    }
}

/// SPAWN-CEILINGS-1 (Codex 4242808144): one wake pass for many players equals their wakes in
/// turn, reads each idle creature's position once, and examines only the creatures in each
/// player's view box, not every creature once per player.
#[test]
fn one_wake_pass_for_many_players_equals_their_wakes_in_turn_and_is_bounded() {
    const PLAYERS: u8 = 40;
    const RATS: usize = 800;
    // Players and creatures share the carrier's slots.
    let mut channel = Channel::with_capacity(usize::from(PLAYERS) + 1 + RATS);
    let mut players = vec![channel.player];
    for tag in 0..PLAYERS {
        let session = GameSessionId::decode(&uuid(10 + tag)).expect("session");
        let reservation = channel
            .runtime
            .reserve_fresh_session(session)
            .expect("reserve");
        let player = channel
            .runtime
            .commit_fresh_session(reservation)
            .expect("commit");
        let (row, column) = (i32::from(tag / 8), i32::from(tag % 8));
        let floor = if tag % 5 == 0 { 6 } else { 7 };
        channel
            .runtime
            .initialize_pinned_test_position(
                player,
                MovementLocalPosition {
                    x: 101 + 4 * column,
                    y: 101 + 6 * row,
                    floor,
                },
            )
            .expect("player position");
        players.push(player);
    }
    // The players stand on odd cells; 600 rats on even cells of floors 6 and 7 around them,
    // 200 far away.
    let mut rats = Vec::new();
    for floor in [6, 7] {
        for x in (100..140).step_by(2) {
            for y in (100..130).step_by(2) {
                let rat = channel
                    .runtime
                    .admit_pinned_test_creature(MovementLocalPosition { x, y, floor })
                    .expect("rat");
                rats.push(rat);
            }
        }
    }
    for index in 0..200 {
        let rat = channel
            .runtime
            .admit_pinned_test_creature(at(10_000 + 2 * index, 100))
            .expect("far rat");
        rats.push(rat);
    }
    assert_eq!(rats.len(), RATS);
    // Three tables with every rat idle: one per wake path.
    let mut tables = [(); 3].map(|()| CreatureAiTable::default());
    for table in &mut tables {
        for rat_actor in &rats {
            table
                .admit_creature(&channel.runtime, *rat_actor, Some(&rat(1)), &abilities(), 0)
                .expect("admitted");
        }
        assert_eq!(
            table
                .run_due_thinks(
                    &mut channel.runtime,
                    0,
                    &channel.root,
                    &channel.revisions,
                    &[],
                    |_| false,
                )
                .len(),
            rats.len()
        );
        assert_eq!(table.pending(), 0);
    }
    let [mut in_turn, mut indexed, mut production] = tables;
    let runtime = &channel.runtime;
    let viewers = players
        .iter()
        .map(|player| channel.position(*player))
        .collect::<Vec<_>>();
    let now = SECOND;

    let expected = viewers
        .iter()
        .flat_map(|viewer| in_turn.wake(*viewer, |rat| live_position(runtime, rat), now))
        .collect::<Vec<_>>();
    assert!(expected.len() > viewers.len());

    let reads = std::cell::Cell::new(0_usize);
    let (woken, examined) = indexed.wake_indexed(
        &viewers,
        |rat| {
            reads.set(reads.get() + 1);
            live_position(runtime, rat)
        },
        now,
    );
    assert_eq!(woken, expected);
    assert_eq!(reads.get(), rats.len());
    let in_box = viewers
        .iter()
        .map(|viewer| {
            rats.iter()
                .map(|rat| channel.position(*rat))
                .filter(|position| {
                    wake_view_box(*viewer, position.floor).is_some_and(|(columns, rows)| {
                        columns.contains(&position.x) && rows.contains(&position.y)
                    })
                })
                .count()
        })
        .sum::<usize>();
    assert_eq!(examined, in_box);
    assert!(examined * 4 < viewers.len() * rats.len());

    assert_eq!(
        production.wake_for_players(&channel.runtime, &players, now),
        expected
    );
    for rat_actor in &rats {
        assert_eq!(production.due(*rat_actor), in_turn.due(*rat_actor));
    }
}
