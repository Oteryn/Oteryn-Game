use super::*;
use oteryn_gameplay_entities::{ChannelContentPin, MovementLocalPosition};
use oteryn_gameplay_foundation::foundation::{ChannelId, NodeId, WorldId};

struct EmptyWorld;
impl SpellWorldReader for EmptyWorld {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<
        Vec<(
            ExactActorRef,
            oteryn_gameplay_foundation::foundation::GameSessionId,
        )>,
    > {
        Some(Vec::new())
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<oteryn_gameplay_content::content::creature_attack_geometry::Facing> {
        Some(oteryn_gameplay_content::content::creature_attack_geometry::Facing::North)
    }
    fn tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: i32,
        _: i32,
        _: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        Some(true)
    }
    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: oteryn_gameplay_foundation::foundation::GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        None
    }
}
struct EmptyStages(EmptyWorld);
impl CallbackStageReaders for EmptyStages {
    fn ready(&self, _: CallbackStageId) -> bool {
        true
    }
    fn reader(&mut self, _: CallbackStageId) -> Option<&mut (dyn SpellWorldReader + '_)> {
        Some(&mut self.0)
    }
}
struct NoPlayerVitals;
impl PlayerLethalVitals for NoPlayerVitals {
    fn apply_attack_damage(
        &mut self,
        _: &mut ChannelRuntimeV1,
        _: ExactActorRef,
        _: oteryn_gameplay_foundation::foundation::GameSessionId,
        _: u32,
        _: &str,
        _: SemanticTimeMicros,
    ) -> Option<crate::player_lethal::PlayerDamageReceipt> {
        None
    }
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn srx3_callback_death_cutoff_preserves_future_original_stage_deadlines() -> Result<(), String> {
    std::thread::Builder::new()
        .name("srx3-cast-death-cutoff".into())
        .stack_size(64 << 20)
        .spawn(run)
        .map_err(debug)?
        .join()
        .map_err(|_| "callback cutoff test thread failed".to_owned())?
}

fn run() -> Result<(), String> {
    let id = |value| {
        let mut bytes = [value; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    };
    let world = WorldId::decode(&id(1)).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        [3; 32],
        [4; 32],
        [5; 32],
        [6; 32],
        (100, 100, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).map_err(debug)?,
        NodeId::decode(&id(3)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        pin,
    )
    .map_err(debug)?;
    let actor = runtime
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.foam_stalker",
            300000,
        )
        .map_err(debug)?;
    let packet: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../apps/game-server/src/source_callback_native_fixture.json"
    ))
    .map_err(debug)?;
    let draft = ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "g4-npc-wave-a-r9".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: hex(world.as_bytes()),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: serde_json::from_value(packet["records"].clone()).map_err(debug)?,
            imports: Vec::new(),
            metadata: Vec::new(),
        },
        state: crate::content::ProjectV2State {
            declarations: serde_json::from_value(packet["declarations"].clone()).map_err(debug)?,
            authoring_profiles: serde_json::from_value(packet["authoring_profiles"].clone())
                .map_err(debug)?,
            source_identity_bindings: serde_json::from_value(
                packet["source_identity_bindings"].clone(),
            )
            .map_err(debug)?,
            sources: serde_json::from_value(packet["sources"].clone()).map_err(debug)?,
            ..Default::default()
        },
    };
    let source = SourceCallbackCast::qualify(
        &draft,
        &Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.foam_stalker".into(),
            revision: "definition-r1".into(),
        },
        4,
        runtime.content_pin().server_artifact_digest(),
    )
    .map_err(debug)?
    .ok_or("actual three-stage Foam source")?;
    let atom = format!("actor:{}", hex(&actor.placement_identity()));
    let proposal = ProfileAbilityProposal {
        issuer: actor,
        target: actor,
        ability: source.parent.clone(),
        list: ScheduleList::Attack,
        entry_index: source.index,
        magnitude: source.magnitude,
        range_tiles: source.range,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!("ai-profile:{}:0:attack:4", hex(&actor.placement_identity())),
            crate::ability::RevisionSet::new(
                "rules-r1",
                "content-r1",
                "policy-r1",
                "definition-r1",
                "sim-r1",
            )
            .map_err(debug)?,
        )
        .map_err(debug)?,
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).map_err(debug)?,
    };
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let mut owner = CallbackCastOwner::new_current(&runtime).map_err(debug)?;
    assert!(
        owner
            .schedule_current(
                &runtime,
                stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0)
            )
            .map_err(debug)?
    );
    let now = SemanticTimeMicros::from_micros(3_500_000);
    assert_eq!(
        owner
            .due_stages_current(&runtime, stamp, now)
            .map_err(debug)?
            .len(),
        3
    );
    let due = owner
        .due_stages_current_through(
            &runtime,
            stamp,
            SemanticTimeMicros::from_micros(1_500_000),
            now,
        )
        .map_err(debug)?;
    assert_eq!(
        due.len(),
        1,
        "the queued death must separate the first source event from later events"
    );
    assert_eq!(due[0].id.stage, 0);
    assert_eq!(
        owner.next_due_time(),
        Some(SemanticTimeMicros::from_micros(1_000_000)),
        "readiness cannot consume or rebase any original deadline"
    );
    let clock = oteryn_gameplay_foundation::foundation::owner_timer::VirtualOwnerClock::new(now);
    let mut readers = EmptyStages(EmptyWorld);
    let mut vitals = NoPlayerVitals;
    let damage = DamageSpellOwner::default();
    let first = owner
        .drain_current_prepared_through(
            &mut runtime,
            stamp,
            &mut vitals,
            &damage,
            &mut readers,
            SemanticTimeMicros::from_micros(1_500_000),
            &clock,
        )
        .map_err(debug)?;
    assert_eq!(
        first.len(),
        1,
        "consumption must obey the same cutoff as readiness"
    );
    assert!(
        first[0].3.is_ok(),
        "an empty current audience is a known source result"
    );
    assert_eq!(
        owner.next_due_time(),
        Some(SemanticTimeMicros::from_micros(2_000_000))
    );
    assert!(
        owner
            .drain_current_prepared_through(
                &mut runtime,
                stamp,
                &mut vitals,
                &damage,
                &mut readers,
                SemanticTimeMicros::from_micros(1_500_000),
                &clock
            )
            .map_err(debug)?
            .is_empty()
    );
    let later = owner
        .drain_current_prepared(
            &mut runtime,
            stamp,
            &mut vitals,
            &damage,
            &mut readers,
            &clock,
        )
        .map_err(debug)?;
    assert_eq!(
        later.len(),
        2,
        "later original deadlines remain available after death ordering"
    );
    assert!(later.iter().all(|row| row.3.is_ok()));
    Ok(())
}
