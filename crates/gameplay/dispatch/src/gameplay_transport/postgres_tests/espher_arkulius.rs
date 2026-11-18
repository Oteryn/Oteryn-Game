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

#[test]
fn espher_arkulius_accepts_the_druid_mission_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("espher-arkulius", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "espher_arkulius", false, false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
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
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
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
    let level = 80;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
        convert_vocation,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    owner.admit_progression_session(&mut admitted).await;
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
    let formula = active
        .native_gameplay()
        .and_then(|native| native.training_formula())
        .ok_or("native training formula")?;
    let before = DurableBuildState::default();
    let after = convert_vocation(formula, &before, "druid").map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "build fence")?
        .ok_or("build fence")?;
    let build = h
        .root
        .commit_character_build(
            &authority,
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
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "experience fence")?
        .ok_or("experience fence")?;
    let award = h
        .root
        .commit_character_experience(
            &authority,
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
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
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
                    level,
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
    for (id, keyword, expected) in [
        (701, "mission", "20 enchanted emeralds"),
        (
            702,
            "yes",
            "Come back here when you have the elemental object",
        ),
    ] {
        let result = talk(&mut owner, &admitted, arkulius, id, keyword).await?;
        assert!(
            result
                .conversation
                .iter()
                .flat_map(|reply| &reply.lines)
                .any(|line| line.contains(expected)),
            "{:?}",
            result.conversation
        );
    }
    let stage: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid
           AND track_key='oteryn:quest-progress/elemental_spheres/quest_line'",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stage, 1);
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
