//! The four Crystal rewards use the existing Arkulius dialogue transaction.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, elemental_spheres,
};
use std::sync::Arc;

const QUEST_LINE: &str = "oteryn:quest-progress/elemental_spheres/quest_line";
const NEUTRAL_MATTER: &str = "oteryn:item.tibia.i954";

#[test]
fn espher2_arkulius_consumes_neutral_matter_and_pays_each_vocation_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    use crate::spell::Vocation::{Druid, Knight, Paladin, Sorcerer};
    super::kill_reward_e2e::scenario_thread::run("espher2-arkulius", || {
        run(async |admin| {
            for (vocation, reward) in [
                (Sorcerer, 8039),
                (Druid, 8041),
                (Paladin, 8025),
                (Knight, 8055),
            ] {
                let h = Harness::create_with_admission_seed(
                    admin.clone(),
                    "espher2_arkulius",
                    true,
                    false,
                )
                .await?;
                let result = exercise(&h, vocation, reward).await;
                h.cleanup().await?;
                result?;
            }
            Ok(())
        })
    })
}

async fn exercise(h: &Harness, vocation: crate::spell::Vocation, reward: u16) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    super::espher2_lord_chamber::record_item_content(
        h,
        pin.identity(),
        pin.frame_binding().digest(),
    )
    .await?;
    let deps = Dependencies::with_content(
        crate::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        ),
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let admitted = deps.admitted;
    let account = super::fixture::id(40);
    let (consumer, clock, _activation, _proof) = purchase_fixture::current(account);
    let premium = SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    owner.active_generation = controller.active();
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
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
                    vocation,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("Arkulius vocation fixture")?;
    }
    let stage_seed = "oteryn:quest-transition/espher2-arkulius-fixture";
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: stage_seed.into(),
                quest: elemental_spheres::QUEST.into(),
                effects: vec![QuestEffect {
                    track: QUEST_LINE.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(2),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    let [arkulius] =
        super::npc_dialogue_fixture::install(&owner, &admitted, ["oteryn:npc.arkulius"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let formula = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("training formula")?;
    let vocation_key = match vocation {
        crate::spell::Vocation::Sorcerer => "sorcerer",
        crate::spell::Vocation::Druid => "druid",
        crate::spell::Vocation::Paladin => "paladin",
        crate::spell::Vocation::Knight => "knight",
        _ => return Err("Arkulius reward vocation".into()),
    };
    let before = DurableBuildState::default();
    let after = convert_vocation(formula, &before, vocation_key).map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                fence,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(super::fixture::id(190))
                        .map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before,
                    after,
                    pruned_stance: None,
                },
                formula,
            )
            .await
            .map_err(debug)?,
        BuildCommitOutcome::Committed(_)
    ));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "post-vocation fence")?
        .ok_or("post-vocation fence")?;
    {
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::foundation::{CommandId, CommandRef};
        let mut revision = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(matches!(
            revision
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: stage_seed.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(699).map_err(debug)?,
                        )),
                    },
                    catalogue,
                )
                .await
                .map_err(debug)?,
            QuestTransitionOutcome::Committed(_)
        ));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::item_look::seed_main_backpack(h, &[(NEUTRAL_MATTER, 2)]).await?;

    super::npc_dialogue_fixture::talk(&mut owner, &admitted, arkulius, 700, "hi").await?;
    super::npc_dialogue_fixture::talk(&mut owner, &admitted, arkulius, 701, "mission").await?;
    super::npc_dialogue_fixture::talk(&mut owner, &admitted, arkulius, 702, "mission").await?;
    let carried: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_item_instances i
         JOIN game_item_container_entries e USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid ORDER BY i.definition_production_key",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        carried,
        vec![
            (format!("oteryn:item.tibia.i{reward}"), 1),
            (NEUTRAL_MATTER.into(), 1)
        ]
    );
    let stage: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(QUEST_LINE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stage, 3);
    Ok(())
}
