//! W15c through real command-7 bytes, native NPC content, Premium and the quest writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{ChannelContentPin, CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::quest_log::{QuestLogLines, QuestLogObservation};
use crate::gameplay_transport::spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_quests::quest::{grizzly_adams as quest, log::QuestLogContent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[path = "grizzly_npc/wire.rs"]
mod wire;
#[path = "grizzly_npc/shop.rs"]
mod shop;
#[path = "grizzly_npc/batches.rs"]
mod batches;
#[path = "grizzly_npc/progression.rs"]
mod progression;
#[path = "grizzly_npc/boss_dialogue.rs"]
mod boss_dialogue;
use shop::rank_shop;

#[test]
fn grizzly_npc_wire_join_choose_cancel_complete_replay_and_journal() -> TestResult {
    postgres_case(0)
}

#[test]
fn grizzly_npc_wire_rank_gate_and_pending_holy_icon_retry_once() -> TestResult {
    postgres_case(1)
}

#[test]
fn grizzly_npc_wire_rank_shop_native_gate_and_purchase_replay_once() -> TestResult {
    postgres_case(2)
}

#[test]
fn grizzly_npc_wire_partial_batch_refusal_refresh_and_unknown_resume() -> TestResult {
    postgres_case(3)
}

#[test]
fn grizzly_npc_wire_task_xp_crosses_level_50_and_unlocks_live_promotion() -> TestResult {
    postgres_case(4)
}

fn postgres_case(case: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("GRIZZLY_NPC_POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    let runner: fn() -> TestResult = match case {
        1 => || scenario(1), 2 => || scenario(2), 3 => || scenario(3), 4 => || scenario(4), _ => || scenario(0),
    };
    super::kill_reward_e2e::scenario_thread::run("grizzly-npc", runner)
}

fn scenario(case: u8) -> TestResult {
    let reward = case == 1;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "grizzly_npc_wire", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        // The native build formula must be activated under the same content
        // interpretation as the fixture's Character root and progression state.
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            world, "content-1",
        )?;
        let active = controller.active().ok_or("active content")?;
        let bundle = wire::world(world)?;
        let cells = bundle.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?;
        let policy = active.native_gameplay().and_then(|native| native.progression())
            .ok_or("native progression")?.policy_for("profile-1", "ruleset-1", "content-1").map_err(debug)?;
        let progression = crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
        let level = if reward { 130 } else if case == 4 { 49 } else { 8 };
        let initial_xp = progression.policy.thresholds.iter().find(|entry|
            entry.level == if case == 4 { 50 } else { level })
            .ok_or("level threshold")?.minimum_experience.get() - if case == 4 { 800 } else { 0 };
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET level=$2,total_experience=$3,simulation_revision=$4,evidence_revision=$5,declaration_revision=$6,policy_revision=$7,reward_revision=$8 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(i64::from(level)).bind(initial_xp)
            .bind(&progression.context.simulation).bind(&progression.context.evidence)
            .bind(&progression.context.declaration).bind(&progression.policy_revision)
            .bind(&progression.reward_revision).execute(&mut *tx).await?;
        tx.commit().await?;
        let channel_pin = ChannelContentPin::from_activation(
            world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(), bundle.frame_binding_digest(world),
            bundle.map_revision_digest(), (2, 0, 7),
        );
        let deps = Dependencies::with_content(channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root.latch_premium_activation(&activation.activation_id, activation.switch_over_us,
            activation.switch_over_us + 60_000_000).await.map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(consumer, Some(clock), Some(activation), Some(h.root.clone()));
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
        let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?.content;
        owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
        owner.quest_log_content = Some(QuestLogLines::new(journal.clone()).ok_or("journal bounds")?);
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"), npc_catalogue_preproduction_limits(),
        ).map_err(debug)?.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        assert!(book.get(quest::NPC).is_some(), "the production catalogue must admit Grizzly");
        owner.states.chat.install_npc_talk(Arc::new(book));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.states.chat.admit(admitted.game_session_id, "Grizzly hunter", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "fence")?.ok_or("fence")?;
        // Use a real durable class for live level refresh, instead of an unrelated
        // process-only cast fixture that cannot be reloaded after quest XP.
        let formula = active.native_gameplay().and_then(|native| native.training_formula()).ok_or("native training")?;
        assert_eq!(formula.content_revision(), progression.context.content,
            "native training and XP share the Character's content interpretation");
        let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
        h.root.commit_character_build(&authority, &h.node, fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                after: crate::durability::character_build::convert_vocation(formula, &build, "druid").map_err(debug)?,
                before: build, pruned_stance: None,
            }, formula).await.map_err(|error| format!("Grizzly NPC durable vocation setup: {error:?}"))?;
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "classed fence")?.ok_or("classed fence")?;
        let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
            crate::gameplay_transport::spell_character_facts::load_character_cast_facts(&h.root, &authority, fence.character_id).await
            else { return Err("durable Druid facts".into()); };
        admitted.item_fence = Some(fence.item_fence());
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
            runtime.bind_attacker_lease(deps.actor, admitted.game_session_id, CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            ).map_err(debug)?).map_err(debug)?;
            let at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
            runtime.place_npcs(&NpcPlacementTable { npcs: vec![NpcPlacements {
                key: quest::NPC.into(), placements: vec![NpcPlacement {
                    x: u16::try_from(at.x + 1)?, y: u16::try_from(at.y)?,
                    floor: -i8::try_from(at.floor)?, direction: MovementFacing::South,
                }],
            }] }, |_| true, true).map_err(debug)?;
            let placed = runtime.visible_entities().npcs.into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some(quest::NPC)).ok_or("Grizzly")?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.initialize(&runtime, deps.actor, admitted.game_session_id,
                facts, (0, 0), owner.owner_now()).is_some());
            EntityRef { identity: placed.actor.placement_identity(), generation: placed.generation }
        };
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned, "native progression admission");
        assert!(owner.progression_binding(admitted.game_session_id).is_some(), "native progression binding");
        assert!(owner.enter_input(admitted.game_session_id).await.is_some(), "fresh quest callback input gate");
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(states.get(&runtime, deps.actor, admitted.game_session_id).is_some(), "exact live actor cast state");
            assert!(!states.is_dead(deps.actor));
        }
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(owner.offered_capabilities(), &[3, 16])
                .ok_or("NPC and journal capabilities")?;
        assert_eq!(admitted.continuity.selected_capabilities.as_slice(), &[3, 16]);
        wire::talk(&mut owner, &mut admitted, npc, &[(1, "hi"), (2, "yes")]).await?;
        assert_eq!(value(&owner, &authority, quest::JOIN_STORAGE).await?, 1);
        assert_eq!(value(&owner, &authority, quest::QUEST_LOG_STORAGE).await?, 0);
        if case == 0 {
            // No native Premium registration: the direct task name cannot bypass Q1.
            wire::talk(&mut owner, &mut admitted, npc,
                &[(3, "hi"), (4, "crocodiles"), (5, "yes")]).await?;
            assert_eq!(value(&owner, &authority, quest::QUEST_STORAGE_BASE + 1).await?, -1);
        }
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), admitted.game_session_id);
        if case == 4 {
            progression::live_level(&mut owner, &mut admitted, npc, &h, &authority, initial_xp).await?;
        } else if case == 3 {
            batches::partial_batches(&mut owner, &mut admitted, npc, &h, &authority, initial_xp).await?;
        } else if case == 2 {
            rank_shop(&mut owner, &mut admitted, npc, &h).await?;
        } else if reward {
            holy_icon(&mut owner, &mut admitted, npc, &h, &authority).await?;
            boss_dialogue::exercise(&mut owner, &mut admitted, npc, &h, &authority).await?;
        } else {
            ordinary_tasks(&mut owner, &mut admitted, npc, &h, &authority, &journal, initial_xp).await?;
        }
        eprintln!("GRIZZLY_NPC_WIRE POSTGRES_CASE_COMPLETED case={case}");
        drop(owner); drop(authority); drop(seal); h.cleanup().await
    })
}

async fn value(owner: &ComposedFreshAdmission<'_, '_, '_>, authority: &ReconciledCharacterAuthority<'_, '_>, storage: u32) -> TestResult<i64> {
    let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
    let copy = owner.root.read_character_quest_state(authority, character).await.map_err(debug)?;
    Ok(quest::storage_value(copy.tracks(), storage))
}

/// Only predecessor counters use fixture privileges; client confirmations and rewards
/// below always pass through the real runtime role, session fences and receipt owners.
async fn seed(h: &Harness, tracks: &[(u32, i64)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (storage, value) in tracks {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(id(41).as_slice()).bind(quest::storage_track(*storage)).bind(quest::QUEST)
            .bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ordinary_tasks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    journal: &QuestLogContent, initial_xp: i64,
) -> TestResult {
    wire::talk(owner, admitted, npc, &[(6, "hi"), (7, "crocodiles"), (8, "yes"), (8, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 1);
    assert_eq!(value(owner, authority, quest::KILL_COUNTER_BASE + 1).await?, 0);
    wire::talk(owner, admitted, npc, &[(9, "hi"), (10, "cancel"), (11, "crocodiles"), (12, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, -1);
    assert_eq!(value(owner, authority, quest::KILLS_STORAGE_BASE + 1).await?, -1);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 1).await?, -1);
    for (id, name) in [(13, "crocodiles"), (16, "badgers"), (19, "tarantulas")] {
        wire::talk(owner, admitted, npc, &[(id, "hi"), (id + 1, name), (id + 2, "yes")]).await?;
    }
    wire::talk(owner, admitted, npc, &[(22, "hi"), (23, "carniphilas"), (24, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 4).await?, -1);
    wire::talk(owner, admitted, npc, &[(25, "hi"), (26, "tasks")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 1);
    seed(h, &[(quest::KILL_COUNTER_BASE + 1, 1)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let QuestLogObservation::Changed(source) = owner.observe_quest_log(
        admitted.runtime_actor.ok_or("actor")?, admitted.game_session_id, None,
    ).await else { return Err("journal source unavailable".into()); };
    let index = journal.active().quests().iter().position(|entry| entry.key == quest::QUEST).ok_or("journal")?;
    let line = journal.line(source.copy.as_ref(), u32::try_from(index)?).ok_or("listed Grizzly")?;
    assert!(line.missions.iter().any(|mission| mission.name == "Paw and Fur: Crocodiles" && mission.text.contains("1/300")));
    seed(h, &[(quest::KILL_COUNTER_BASE + 1, 300), (quest::KILL_COUNTER_BASE + 2, 300)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(27, "hi"), (28, "tasks"), (28, "tasks"), (29, "tasks")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 0);
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 2).await?, 0);
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 3).await?, 1);
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 2);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 1).await?, 1);
    assert_eq!(value(owner, authority, quest::boss(1).ok_or("boss")?.storage).await?, 1);
    let (completed, experience): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key IN ('oteryn:quest-transition/grizzly/complete-1','oteryn:quest-transition/grizzly/complete-2')), (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid)",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(completed, 2);
    assert_eq!(experience, initial_xp + 1300);
    owner.states.quest_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 2);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 1).await?, 1);
    Ok(())
}

async fn holy_icon(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    wire::talk(owner, admitted, npc, &[(3, "hi"), (4, "demons"), (5, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 42).await?, -1);
    seed(h, &[(quest::POINTS_STORAGE, 100), (quest::PROMOTION_STORAGE, 7)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(6, "hi"), (7, "demons"), (8, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 42).await?, 1);
    seed(h, &[(quest::KILL_COUNTER_BASE + 42, 6666)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(9, "hi"), (10, "tasks")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 42).await?, 2);
    let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
    assert!(h.root.pending_grizzly_reward(authority, character).await.map_err(debug)?.is_some(),
        "no backpack preserves the earned icon");
    super::item_look::seed_main_backpack(h, &[]).await?;
    owner.states.quest_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    wire::talk(owner, admitted, npc, &[(11, "hi"), (12, "status"), (12, "status")]).await?;
    assert!(h.root.pending_grizzly_reward(authority, character).await.map_err(debug)?.is_none());
    let icons: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9388'")
        .fetch_one(&h.pool).await?;
    assert_eq!(icons, 1);
    assert_eq!(value(owner, authority, 41300).await?, 1);
    let rewards: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-42'")
        .fetch_one(&h.pool).await?;
    assert_eq!(rewards, 1);
    Ok(())
}
