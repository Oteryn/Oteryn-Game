#![allow(clippy::expect_used)]
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{ProfileScheduleInput, ProfileScheduleState},
};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_attack_geometry::Facing;
use crate::creature_auto_attack::AttackError;
use crate::creature_damage_spell::{SpellPresentationReader, SpellWorldReader};
use crate::creature_defense_presentation::*;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1, RuntimeWorkStamp,
};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates,
    tests::{FACTS, runtime_with_player},
};

fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/creature_auto_attack_test_data.json"
    ))
    .expect("qualified fixture");
    (
        serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
        serde_json::from_value(v["authoring_profiles"].clone()).expect("qualified fixture"),
    )
}
struct TilePolicy {
    missing: bool,
}
impl SpellWorldReader for TilePolicy {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, crate::foundation::GameSessionId)>> {
        None
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        None
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        _: i32,
        _: i32,
        _: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        r.read_actor_position(a).ok()?;
        (!self.missing).then_some(true)
    }
    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: crate::foundation::GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<crate::creature_damage_spell::SpellCombatFacts> {
        None
    }
}
#[test]
fn all_twelve_actual_native_defense_presentation_sources_qualify() {
    let (records, profiles) = native();
    let packet: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/creature_defense_presentation_test_data.json"
    ))
    .expect("qualified fixture");
    let mut count = 0;
    for row in packet["rows"].as_array().expect("qualified fixture") {
        let c: Ref = serde_json::from_value(row["creature"].clone()).expect("qualified fixture");
        DefensePresentationSource::from_native(
            &c,
            row["index"].as_u64().expect("qualified fixture") as usize,
            &records,
            &profiles,
            [1; 32],
        )
        .expect("qualified fixture");
        count += 1;
    }
    assert_eq!(count, 12);
}
#[test]
fn actual_self_defense_schedule_emits_at_caster_without_player_target_or_hp_mp() {
    let (mut r, player, session) = runtime_with_player(0x58);
    r.initialize_movement_test_position(
        player,
        MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
    )
    .expect("qualified fixture");
    let (records, profiles) = native();
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.blood_hand".into(),
        revision: "definition-r1".into(),
    };
    let hp = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target == creature => c.health,
            _ => None,
        })
        .expect("qualified fixture");
    let actor = r
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 103,
                y: 104,
                floor: 7,
            },
            &creature.key,
            hp as i64,
        )
        .expect("qualified fixture");
    let source = DefensePresentationSource::from_native(
        &creature,
        1,
        &records,
        &profiles,
        r.content_pin().server_artifact_digest(),
    )
    .expect("qualified fixture");
    let behavior = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == "oteryn:behavior.creature.blood_hand" => Some(b),
            _ => None,
        })
        .expect("qualified fixture");
    let abilities = profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let mut schedule = ProfileScheduleState::new(actor);
    let mut tracker = ThinkSequenceTracker::new();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
        r.content_pin().server_artifact_digest(),
    );
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let mut found = None;
    for _ in 0..256 {
        let plan = schedule
            .prepare_with_summons(
                ProfileScheduleInput {
                    occurrence: tracker.next_occurrence(actor),
                    behavior,
                    abilities: &abilities,
                    target: None,
                    revisions: &revisions,
                    root: &root,
                },
                None,
            )
            .expect("qualified fixture");
        if let Some(p) = plan
            .proposals
            .into_iter()
            .find(|p| p.ability == source.ability)
        {
            found = Some(p);
            break;
        }
    }
    let p = found.expect("actual defense schedule supports no player target");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            player,
            session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let b = r.binding();
    let (mut f, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    let mut policy = TilePolicy { missing: false };
    let event = source
        .produce(&r, &f, stamp, &p, &mut SpellPresentationReader(&mut policy))
        .expect("qualified fixture");
    assert!(event.tiles.contains(&(103, 104, 7)));
    assert!(event.source.impact_cue.is_some());
    let (revision, vitals) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session)
            .expect("qualified fixture");
    assert_eq!((revision, vitals.health, vitals.mana), (1, 185, 90));
    policy.missing = true;
    assert_eq!(
        source.produce(&r, &f, stamp, &p, &mut SpellPresentationReader(&mut policy)),
        Err(AttackError::MissingCombatFacts)
    );
    f.apply_external_grant(
        crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
    )
    .expect("qualified fixture");
    assert_eq!(
        source.produce(&r, &f, stamp, &p, &mut SpellPresentationReader(&mut policy)),
        Err(AttackError::StaleOwner)
    );
}
