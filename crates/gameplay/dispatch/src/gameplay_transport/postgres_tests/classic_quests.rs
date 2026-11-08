//! Real NPC command routing and the existing durable QuestState writer.
//! This test is red without the batch-2 adapter: a static keyword reply cannot create a receipt.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const MORRIS: &str = "oteryn:npc.mr_morris";
const WOBLIN: &str = "oteryn:npc.woblin";
const JEROM: &str = "oteryn:npc.jerom";
const QUEEN: &str = "oteryn:npc.the_queen_of_the_banshees";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const DAWN: &str = "oteryn:quest-progress/quest/u10_55/dawnport/";
const TROLL: &str = "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline";

#[test]
fn classic_quests_npc_commands_commit_exact_start_branches_and_fence_topics() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_QUESTS_NPC_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("classic-quests-npc", || {
        super::quest_world_callbacks::scenario(
            "classic-quests-npc",
            Position { x: 33294, y: 31677, floor: -15 },
            false,
        )
    })
}

pub(super) async fn install<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, |i| (i, 1)).await
}

pub(super) async fn install_above<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession, keys: [&str; N],
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, |i| (i, -1)).await
}

async fn install_offsets<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession, keys: [&str; N],
    offset: impl Fn(i32) -> (i32, i32),
) -> TestResult<[EntityRef; N]> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"), npc_catalogue_preproduction_limits(),
    ).map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(NpcTalkBook::from_model_with_native_quests(&model, &keys)));
    owner.states.chat.admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime.bind_attacker_lease(actor, admitted.game_session_id,
        crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        ).map_err(debug)?).map_err(debug)?;
    let at = runtime.read_actor_position(actor).map_err(debug)?.position();
    let mut placements = keys.into_iter().enumerate().map(|(i,key)| {
            let (dx,dy) = offset(i32::try_from(i)?);
            Ok(NpcPlacements { key: key.into(), placements: vec![NpcPlacement {
                x: u16::try_from(at.x + dx)?,
                y: u16::try_from(at.y + dy)?,
                floor: -i8::try_from(at.floor)?,
                direction: MovementFacing::South,
            }]})
        }).collect::<TestResult<Vec<_>>>()?;
    placements.sort_by(|left, right| left.key.cmp(&right.key));
    runtime.place_npcs(&NpcPlacementTable {
        npcs: placements,
    }, |_| true, true).map_err(debug)?;
    let mut found = Vec::new();
    for key in keys {
        found.push(runtime.visible_entities().npcs.into_iter().find_map(|npc| {
            (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                identity: npc.actor.placement_identity(), generation: npc.generation,
            })
        }).ok_or("placed source NPC")?);
    }
    found.try_into().map_err(|_| "exact NPC refs".into())
}

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession,
    npc: EntityRef, command_id: u64, text: &str,
) -> TestResult<Vec<String>> {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let result = owner.npc_talk(admitted.runtime_actor.ok_or("actor")?, UseCommand {
        game_session_id: admitted.game_session_id, command_id, item_fence: Some(fence.item_fence()),
    }, NpcTalkIntent { npc_actor: npc, text: text.into() }).await;
    assert_eq!(result.disposition, NpcIntentDisposition::Ok, "{command_id}: {text}");
    assert!(!result.unknown);
    Ok(result.conversation.map(|c| c.lines).unwrap_or_default())
}

pub(super) async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>, track: &str,
) -> TestResult<i64> {
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let copy = owner.root.read_character_quest_state(authority, fence.character_id).await.map_err(debug)?;
    Ok(copy.tracks().get(track).copied().unwrap_or(0))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [morris,woblin,jerom,queen] = install(owner, admitted, [MORRIS,WOBLIN,JEROM,QUEEN]).await?;
    talk(owner, admitted, morris, 1200, "hi").await?;
    talk(owner, admitted, morris, 1201, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}the_lost_amulet")).await?, 0);
    let offer = talk(owner, admitted, morris, 1202, "amulet").await?;
    assert!(offer.iter().any(|line| line.contains("ancient amulet")));
    let accepted = talk(owner, admitted, morris, 1203, "yes").await?;
    assert!(accepted.iter().any(|line| line.contains("Dormovo alive")));
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}the_lost_amulet")).await?, 1);
    talk(owner, admitted, morris, 1204, "yes").await?;
    talk(owner, admitted, morris, 1205, "herbs").await?;
    let herbs = talk(owner, admitted, morris, 1206, "yes").await?;
    assert_eq!(herbs.len(), 2, "both literal donor reply parts must be retained");
    assert!(herbs[1].contains("light grey-brown sand"));
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}the_rare_herb")).await?, 1);
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}herb_flower")).await?, 1);
    talk(owner, admitted, morris, 1207, "key").await?;
    talk(owner, admitted, morris, 1208, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}the_dorm_key")).await?, 1);
    talk(owner, admitted, woblin, 1209, "hi").await?;
    let fish = talk(owner, admitted, woblin, 1210, "precious reward").await?;
    assert_eq!(fish.len(), 2, "Woblin has consecutive source ifs");
    assert!(fish[1].contains("Old Nasty"));
    assert_eq!(progress(owner, admitted, authority, &format!("{DAWN}the_dorm_key")).await?, 2);
    // An offer made to Morris is not a Jerom confirmation.
    talk(owner, admitted, morris, 1211, "hi").await?;
    talk(owner, admitted, morris, 1212, "log book").await?;
    talk(owner, admitted, jerom, 1213, "hi").await?;
    talk(owner, admitted, jerom, 1214, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, TROLL).await?, 0);
    let mission = talk(owner, admitted, jerom, 1215, "mission").await?;
    assert_eq!(mission.len(), 3);
    talk(owner, admitted, jerom, 1216, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, TROLL).await?, 1);
    talk(owner, admitted, jerom, 1217, "yes").await?;
    talk(owner, admitted, queen, 1218, "hi").await?;
    talk(owner, admitted, queen, 1219, "seventh seal").await?;
    let refused = talk(owner, admitted, queen, 1220, "yes").await?;
    assert!(refused.iter().any(|line| line.contains("not passed the seal of sacrifice")));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/classic/%'")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts, 5, "five mutations, never a receipt for static answers or repeated yes");
    Ok(())
}

#[test]
fn classic_rottin_disrepair_commits_three_flags_only_after_the_source_offer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_ROTTIN_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("classic-rottin", || {
        super::quest_world_callbacks::scenario(
            "classic-rottin-disrepair", Position { x: 33294, y: 31677, floor: -15 }, false,
        )
    })
}

pub(super) async fn exercise_rottin(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
    use crate::foundation::{CommandId, CommandRef};
    const TRACK: &str = "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/";
    let [rottin] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, rottin, 1700, "hi").await?;
    talk(owner, admitted, rottin, 1701, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{TRACK}questline")).await?, 0);
    // Establish only the test's prior completed mission through its existing
    // source transition; this is not a production path around the deferred hand-in.
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot.commit_quest_transition(owner.root, owner.character, owner.holder, fence,
        QuestTransitionRequest {
            transition_key: "oteryn:quest-transition/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline/npc_1".into(),
            cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(1702).map_err(debug)?)),
        }, owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone()).await.map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let offered = talk(owner, admitted, rottin, 1703, "mission").await?;
    assert_eq!(offered.len(), 5);
    assert!(offered[1].contains("deer stands"));
    let accepted = talk(owner, admitted, rottin, 1704, "yes").await?;
    assert!(accepted.iter().any(|line| line.contains("it was not made by us")));
    assert_eq!(progress(owner, admitted, authority, &format!("{TRACK}questline")).await?, 3);
    assert_eq!(progress(owner, admitted, authority, &format!("{TRACK}mission02")).await?, 1);
    assert_eq!(progress(owner, admitted, authority, "oteryn:quest-progress/crystalserver/kv/rottinwood-wallcount").await?, 0);
    talk(owner, admitted, rottin, 1705, "yes").await?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/classic/rottin-wood/start-disrepair'")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    // The donor refuses an incomplete repair without changing state or money.
    // A completed repair's combined reward remains deferred to its shared owner.
    talk(owner, admitted, rottin, 1706, "mission").await?;
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let refused = owner.npc_talk(admitted.runtime_actor.ok_or("actor")?, UseCommand {
        game_session_id: admitted.game_session_id, command_id: 1707, item_fence: Some(fence.item_fence()),
    }, NpcTalkIntent { npc_actor: rottin, text: "yes".into() }).await;
    assert_eq!(refused.disposition, NpcIntentDisposition::Ok);
    assert!(!refused.inventory_changed);
    assert!(refused.conversation.is_some_and(|reply|
        reply.lines.iter().any(|line| line.contains("walls haven't been repaired"))));
    assert_eq!(progress(owner, admitted, authority, &format!("{TRACK}questline")).await?, 3);
    Ok(())
}

#[test]
fn classic_draconia_real_levers_open_and_restore_the_source_obstacles() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_DRACONIA_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("classic-draconia", || {
        super::quest_world_callbacks::scenario(
            "draconia-lever-30006", Position { x: 32792, y: 31595, floor: -7 }, false,
        )?;
        super::quest_world_callbacks::scenario(
            "draconia-lever-30035", Position { x: 32792, y: 31579, floor: -7 }, false,
        )
    })
}

#[test]
fn classic_world_steps_use_source_teleports_and_commit_seal_flags_atomically() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_STEPS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("classic-steps", || {
        for (name, trigger) in [
            ("outlaw-camp-entrance", Position { x: 32619, y: 32248, floor: -6 }),
            ("banshees-first-seal-return", Position { x: 32266, y: 31885, floor: -12 }),
            ("banshees-second-seal", Position { x: 32171, y: 31853, floor: -15 }),
            ("banshees-final-battle-exit", Position { x: 32219, y: 31913, floor: -15 }),
        ] {
            super::quest_world_callbacks::scenario(name, trigger, false)?;
        }
        Ok(())
    })
}

#[allow(clippy::too_many_arguments, reason = "existing callback fixture owner and source binding")]
pub(super) async fn exercise_step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    world: &crate::map::boot::BundleWorld,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    trigger: Position, origin: crate::foundation::MovementLocalPosition,
) -> TestResult {
    let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    );
    super::quest_world_callbacks::wire(owner, admitted, 1, 1, step.clone()).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
        super::quest_world_callbacks::local(callback.teleport.ok_or("teleport")?));
    for effect in &callback.storage {
        assert_eq!(progress(owner, admitted, authority, &effect.track).await?, effect.set);
    }
    if callback.key.ends_with("banshees-second-seal") {
        {
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            let doors = crate::map::floor::BundleDoors(world.facts().doors());
            let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
                &runtime, owner.movement_cells(), actor, admitted.game_session_id,
                expected, origin, false, &doors,
                None,
            ).map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
        super::quest_world_callbacks::wire(owner, admitted, 2, 1, step).await?;
        assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), origin,
            "a completed Second Seal must return to the previous tile");
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
            .bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1);
    }
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1).ok_or("source key")?;
    assert!(world.facts().quest_callback_matches(key, callback.selector.unique, callback.selector.action, callback.selector.appearance));
    Ok(())
}
