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
        _: &ChannelRuntimeV1,
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
        let world = WorldId::decode(&uuid(1)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).expect("channel"),
            NodeId::decode(&uuid(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
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
        &self,
        melee: &mut MonsterMeleeOwner,
        vitals: &mut Vitals,
        creature: ExactActorRef,
        sequence: u64,
        now_us: u64,
    ) -> Dispatch {
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
        let target = self.player;
        let session = self.session;
        melee
            .think(
                &self.runtime,
                vitals,
                creature,
                sequence,
                definition,
                input,
                &[PerceivedPlayer {
                    id: PerceivedPlayerId::new(1),
                    position: self.position(self.player),
                    legal_attack_target: true,
                }],
                |_| {
                    Some((
                        target,
                        session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                self.revisions.clone(),
                SemanticTimeMicros::from_micros(now_us),
            )
            .expect("swing")
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
