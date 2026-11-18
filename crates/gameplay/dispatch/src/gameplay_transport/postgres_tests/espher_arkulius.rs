//! Arkulius mission acceptance through the fenced dialogue dispatcher.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MovementFacing, NpcPlacement,
    NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const ARKULIUS: &str = "oteryn:npc.arkulius";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mission {
    Published,
    Expanded,
    ExpandedWithSoil,
}

#[test]
fn espher_arkulius_accepts_the_druid_mission_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(Mission::Published)
}

#[test]
fn espher_arkulius_checks_eleven_item_identities_in_the_npc_transaction_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(Mission::Expanded)
}

#[test]
fn espher_arkulius_refuses_start_with_carried_soil_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(Mission::ExpandedWithSoil)
}

fn run_case(mission_fixture: Mission) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("espher-arkulius", move || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "espher_arkulius", false, false).await?;
            let result = exercise(&h, mission_fixture).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness, mission_fixture: Mission) -> TestResult {
    prepare_current_session(h).await?;
    let carries_soil = mission_fixture == Mission::ExpandedWithSoil;
    if carries_soil {
        super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i940", 1)]).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (2, 0, 7),
    );
    let deps = Dependencies::with_content_and_capacity(
        channel_pin,
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    let mut admitted = deps.admitted;
    let premium = premium_fixture(h).await?;
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    owner.active_generation = Some(active);
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let arkulius = install(
        &owner,
        &admitted,
        NpcPlacement {
            x: 2,
            y: 1,
            floor: -7,
            direction: MovementFacing::South,
        },
    )
    .await?;
    admit_level_80_druid(&owner, &mut admitted, h, &authority).await?;
    let catalogue = if mission_fixture == Mission::Published {
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone()
    } else {
        eleven_item_catalogue()?
    };
    owner.quest_catalogue = Some(std::sync::Arc::new(catalogue));
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    {
        let runtime = deps.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    level: 80,
                    vocation: crate::spell::Vocation::Druid,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("fixture level projection")?;
    }
    owner.admit_premium(id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    talk(&mut owner, &admitted, arkulius, 700, "hi").await?;
    let mission = talk(&mut owner, &admitted, arkulius, 701, "mission").await?;
    assert!(
        mission
            .conversation
            .iter()
            .flat_map(|reply| &reply.lines)
            .any(|line| line.contains("20 enchanted emeralds")),
        "{:?}",
        mission.conversation
    );
    let result = talk(&mut owner, &admitted, arkulius, 702, "yes").await?;
    let accepted = result
        .conversation
        .iter()
        .flat_map(|reply| &reply.lines)
        .any(|line| line.contains("Come back here when you have the elemental object"));
    assert_eq!(accepted, !carries_soil, "{:?}", result.conversation);
    let stage: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid
           AND track_key='oteryn:quest-progress/elemental_spheres/quest_line'",
    )
    .bind(id(41).as_slice())
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(stage, (!carries_soil).then_some(1));
    Ok(())
}

// The published programme contains five item identities. Extend its acceptance
// rule in memory to drive the eleven-item inventory plan through dialogue dispatch.
fn eleven_item_catalogue() -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::elemental_spheres::{QUEST, QUEST_LINE};
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };

    const ACCEPT: &str = "oteryn:quest-npc/elemental_spheres/arkulius/druid-accept";
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/dialogues/quest-rules/quest-batch3-a-elemental_spheres.json"
    ))?;
    let guards = document["rules"]
        .as_array_mut()
        .and_then(|rules| rules.iter_mut().find(|rule| rule["key"] == ACCEPT))
        .and_then(|rule| rule["guards"].as_array_mut())
        .ok_or("Arkulius acceptance guards")?;
    guards.extend(
        [940, 941, 942, 943, 944, 945, 946, 947, 948, 51259].map(|id| {
            serde_json::json!({
                "kind": "item", "item": format!("oteryn:item.tibia.i{id}"), "op": "lt", "count": 1
            })
        }),
    );
    let document = document.to_string();
    let source = NpcRuleCatalog::from_json_documents(&[&document]).map_err(debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: QUEST_LINE.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 3,
        }],
        vec![QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: QUEST_LINE.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?
    .with_fixture_npc_program(document))
}

pub(super) async fn premium_fixture(
    h: &Harness,
) -> TestResult<crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator> {
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    Ok(SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    ))
}

pub(super) async fn admit_level_80_druid(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let level = 80;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
        convert_vocation,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    owner.admit_progression_session(admitted).await;
    let progression = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("native progression")?;
    let experience = progression
        .policy
        .thresholds
        .iter()
        .find(|entry| entry.level == level)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("native training formula")?;
    let before = DurableBuildState::default();
    let after = convert_vocation(formula, &before, "druid").map_err(debug)?;
    let fence =
        super::death_ml_loss_composed::current_fence(owner, admitted.game_session_id).await?;
    let build = h
        .root
        .commit_character_build(
            authority,
            &h.node,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(build, BuildCommitOutcome::Committed(_)),
        "{build:?}"
    );
    let fence =
        super::death_ml_loss_composed::current_fence(owner, admitted.game_session_id).await?;
    let award = h
        .root
        .commit_character_experience(
            authority,
            &h.node,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(i64::try_from(experience)?),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(award, ExperienceCommitOutcome::Committed(_)),
        "{award:?}"
    );
    Ok(())
}

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    placement: NpcPlacement,
) -> TestResult<EntityRef> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[ARKULIUS]),
    ));
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("current fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: ARKULIUS.into(),
                    placements: vec![placement],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some(ARKULIUS))
        .ok_or("Arkulius placement")?;
    Ok(EntityRef {
        identity: npc.actor.placement_identity(),
        generation: npc.generation,
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: EntityRef,
    id: u64,
    text: &str,
) -> TestResult<oteryn_gameplay_npc::npc_command::NpcTalkOutcome> {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("current item fence")?;
    let reply = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: target,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "{text}");
    assert!(!reply.unknown, "{text}");
    Ok(reply)
}
