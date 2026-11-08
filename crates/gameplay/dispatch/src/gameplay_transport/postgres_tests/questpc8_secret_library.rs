//! Selected library journey: real NPC discussions, owned component conversion and captured Oberon death.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[path = "questpc8_secret_library/world.rs"]
mod fixture_world;

#[test]
fn questpc8_secret_library_raft_research_fiddle_gail_oberon_and_access() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-secret-library", || {
        quest_world_callbacks::scenario(
            "authored-secret-library",
            Position {
                x: 32031,
                y: 32463,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_secret_library_raft_research_fiddle_gail_oberon_and_access duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    fixture_world::world()
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3],
        )
        .ok_or("native NPC capability")?;
    assert_eq!(admitted.continuity.selected_capabilities.as_slice(), &[3]);
    let premium = owner.premium_coordinator.take();
    let mut command = 1;
    raise_level(owner, admitted, 249, 201).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, -1);
    raise_level(owner, admitted, 250, 202).await?;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        progress(h, "stage").await?,
        -1,
        "premium remains required at level250"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 0);
    let [charles, spectulus, gail] = install_npcs(owner, admitted).await?;
    travel(owner, admitted, 32529, 32782, 6).await?;
    say(owner, admitted, charles, &mut command, &["hi", "raft"]).await?;
    assert_eq!(progress(h, "stage").await?, 1);
    travel(owner, admitted, 32030, 32463, 7).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32031, 32463, -7, 0).await?,
        UseDisposition::Rejected,
        "hawser is required"
    );
    carry(owner, admitted, 28707, 501).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32031, 32463, -7, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        quantity(h, 28707).await?,
        0,
        "raft binds and consumes the owned hawser"
    );
    assert_eq!(progress(h, "stage").await?, 2);
    travel(owner, admitted, 33268, 31834, 1).await?;
    say(owner, admitted, spectulus, &mut command, &["hi", "rumours"]).await?;
    assert_eq!(progress(h, "stage").await?, 3);
    travel(owner, admitted, 33524, 31464, 14).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 4);
    travel(owner, admitted, 32807, 32764, 10).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32807, 32765, -10, 0).await?,
        UseDisposition::Rejected,
        "missing ebony wood cannot assemble an instrument"
    );
    carry(owner, admitted, 28491, 502).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32808, 32765, -10, 1).await?,
        UseDisposition::Rejected,
        "hair station cannot precede fingerboard"
    );
    assert_eq!(
        claim(owner, admitted, &mut command, 32807, 32765, -10, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h, 28491).await?, 0);
    assert_eq!(quantity(h, 28492).await?, 1);
    refresh(owner, admitted).await;
    let target = object_target(
        owner,
        TilePos {
            x: 32808,
            y: 32765,
            floor: -10,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed,
        "same source conversion replays once"
    );
    command += 1;
    assert_eq!(quantity(h, 28492).await?, 0);
    assert_eq!(quantity(h, 28493).await?, 1);
    assert_eq!(progress(h, "stage").await?, 6);
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let denied = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (denied.x, denied.y, denied.floor),
        (32671, 32736, 11),
        "unearned native entrance returns to the source approach"
    );
    assert_eq!(
        progress(h, "access").await?,
        -1,
        "instrument alone does not grant library access"
    );
    travel(owner, admitted, 32621, 32737, 6).await?;
    say(owner, admitted, gail, &mut command, &["hi", "daughter"]).await?;
    assert_eq!(progress(h, "stage").await?, 7);
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let denied = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (denied.x, denied.y, denied.floor),
        (32671, 32736, 11),
        "unearned native entrance returns to the source approach"
    );
    assert_eq!(
        progress(h, "access").await?,
        -1,
        "Gail report does not replace Oberon death"
    );
    travel(owner, admitted, 33364, 31318, 9).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actors = owner
        .quest_guardians_for_test("oteryn:quest-death/authored-secret-library-oberon")
        .await;
    assert_eq!(actors.len(), 1);
    super::grizzly_npc::questpc8_campaign::lethal(
        owner,
        admitted,
        actors[0],
        "oteryn:creature.grand_master_oberon",
        command,
        1,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "kills").await?, 1);
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (position.x, position.y, position.floor),
        (32480, 32597, 15),
        "earned access operates the original native library descent"
    );
    travel(owner, admitted, 32479, 32597, 15).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 8);
    assert_eq!(progress(h, "access").await?, 1);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    assert_eq!(
        quantity(h, 28493).await?,
        1,
        "final access and cold replay do not duplicate the instrument"
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_secret_library_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn carry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    occurrence: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 32030,
                y: 32463,
                floor: -7,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        occurrence,
        true,
    )
    .await
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/secret-library/{suffix}")).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: u16,
    y: u16,
    floor: i8,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    refresh(owner, admitted).await;
    let target = object_target(owner, TilePos { x, y, floor }, ordinal)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}
async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    text: &[&str],
) -> TestResult {
    refresh(owner, admitted).await;
    let commands = text
        .iter()
        .enumerate()
        .map(|(offset, text)| {
            Ok((
                command
                    .checked_add(u64::try_from(offset)?)
                    .ok_or("command")?,
                *text,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    super::grizzly_npc::wire::talk(owner, admitted, npc, &commands).await?;
    *command = command
        .checked_add(u64::try_from(text.len())?)
        .ok_or("command")?;
    Ok(())
}

// The live owner cadence reloads scheduled quest state. Explicitly perform that same
// durable read between fixture commands so cross-owner selection never reads an old copy.
async fn refresh(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
}

/// Fixture travel between genuine objective areas uses the existing sealed teleport
/// proof, as classic_quests does; it never seeds or advances any quest track.
async fn travel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    x: i32,
    y: i32,
    floor: i16,
) -> TestResult {
    refresh(owner, admitted).await;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        crate::foundation::MovementLocalPosition { x, y, floor },
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn install_npcs(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 3]> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let keys = [
        "oteryn:npc.charles",
        "oteryn:npc.spectulus",
        "oteryn:npc.gail",
    ];
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &keys),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Library researcher",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut npcs = keys
        .into_iter()
        .zip([(32529, 32783, -6), (33268, 31835, -1), (32621, 32738, -6)])
        .map(|(key, (x, y, floor))| NpcPlacements {
            key: key.into(),
            placements: vec![NpcPlacement {
                x,
                y,
                floor,
                direction: MovementFacing::South,
            }],
        })
        .collect::<Vec<_>>();
    npcs.sort_by(|a, b| a.key.cmp(&b.key));
    runtime
        .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
        .map_err(debug)?;
    let refs = keys
        .into_iter()
        .map(|key| {
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|npc| {
                    (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
                })
                .ok_or_else(|| "source NPC".into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    refs.try_into().map_err(|_| "exact NPC refs".into())
}
async fn raise_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    level: u32,
    occurrence: u8,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let xp = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == level)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let amount = xp.checked_sub(current.total_experience.get()).ok_or("XP")?;
    if amount > 0 {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(amount),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    }
    Ok(())
}
