use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug, id};

use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};

use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};

use crate::map::overlay::TilePos;

use oteryn_protocol_oteryn::{
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    world_object::{
        CAPABILITY_ITEM_USE_V1, CAPABILITY_WORLD_INTERACTION_V1, MapUseTarget, UseDisposition,
        UseIntent, UseTarget, encode_item_use_intent,
    },
    world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
};

use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[path = "questpc8_magic_sword/world.rs"]
mod fixture_world;

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active content")?;
    fixture_world::world(active.native_gameplay().ok_or("native world items")?)
}

const UPPER: &str = "oteryn:quest-death/authored-magic-sword-upper";

const LOWER: &str = "oteryn:quest-death/authored-magic-sword-lower";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_WORLD_INTERACTION_V1,
            CAPABILITY_ITEM_USE_V1,
            CAPABILITY_WORLD_MAP_VIEW_V1,
        ],
    )
    .ok_or("numbered key capabilities")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mut command = 1;
    raise_level(owner, admitted, 99, 80).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, -1);
    assert!(owner.quest_guardians_for_test(UPPER).await.is_empty());
    raise_level(owner, admitted, 100, 81).await?;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(progress(h, "upper").await?, 0);
    assert!(owner.quest_guardians_for_test(LOWER).await.is_empty());
    clear_group(owner, admitted, UPPER, 2, command).await?;
    assert_eq!(progress(h, "upper").await?, 2);
    walk(owner, admitted, &mut command, 32575, 32198).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        position(owner, admitted).await?.floor,
        15,
        "real STEP descends the native432 hole"
    );
    walk(owner, admitted, &mut command, 32578, 32198).await?;
    let before_door = position(owner, admitted).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let blocked = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&blocked).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Blocked
    );
    command += 1;
    assert_eq!(
        position(owner, admitted).await?,
        before_door,
        "the numbered lock blocks entry before it is opened"
    );
    let door_position = TilePos {
        x: 32578,
        y: 32197,
        floor: -15,
    };
    let door = object_target(owner, door_position, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &door).await?,
        UseDisposition::Locked,
        "upper-chamber progress cannot substitute for the numbered key"
    );
    command += 1;
    use_numbered_key(owner, admitted, h, &mut command, door_position).await?;
    walk(owner, admitted, &mut command, 32578, 32196).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "stage").await?, 2);
    let north = TilePos {
        x: 32577,
        y: 32193,
        floor: -15,
    };
    let south = TilePos {
        x: 32578,
        y: 32193,
        floor: -15,
    };
    // The lower chamber has four live native actors; no manually created kill targets.
    assert_eq!(owner.quest_guardians_for_test(LOWER).await.len(), 4);
    walk(owner, admitted, &mut command, 32578, 32194).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Rejected,
        "the first cache requires all four captured lower-chamber deaths"
    );
    assert_eq!(progress(h, "lower").await?, 0);
    clear_group(owner, admitted, LOWER, 4, command).await?;
    assert_eq!(progress(h, "lower").await?, 4);
    walk(owner, admitted, &mut command, 32578, 32194).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected,
        "the second cache cannot pay first"
    );
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h, "stage").await?, 3);
    for (item, count) in [(651, 1), (3388, 1), (3389, 1), (3288, 0)] {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Rejected
    );
    let target = object_target(owner, south, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(progress(h, "stage").await?, 4);
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected
    );
    for (item, count) in [(651, 1), (3388, 2), (3389, 2), (3288, 1)] {
        assert_eq!(quantity(h, item).await?, count);
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.magic_sword_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn use_numbered_key(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    command: &mut u64,
    position: TilePos,
) -> TestResult {
    let reward = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/l6-mad-mage-prisoner-key")
        .and_then(|callback| callback.give)
        .ok_or("Prisoner's key reward")?;
    // Seed the published reward into the starter backpack. The door journey
    // exercises the real carried-item lookup and native key transition.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seeded = sqlx::query(
        "WITH carried_key AS (
             INSERT INTO game_item_instances(item_instance_id, world_id, definition_family,
                 definition_production_key, definition_revision_ref, quantity, lifecycle,
                 minted_transaction_id)
             VALUES (game_character_uuid_v7(), encode($1,'hex')::uuid, 'Item', $2,
                 'definition-r1', $3, 1, game_character_uuid_v7())
             RETURNING item_instance_id, world_id, minted_transaction_id
         ), numbered AS (
             INSERT INTO game_item_keys(item_instance_id, key_number, minted_transaction_id)
             SELECT item_instance_id, $4, minted_transaction_id FROM carried_key
         )
         INSERT INTO game_item_container_entries(character_id, world_id, parent_item_instance_id,
             item_instance_id, placement_ordinal, placed_transaction_id)
         SELECT pack.character_id, pack.world_id, pack.item_instance_id, k.item_instance_id,
             1, k.minted_transaction_id
         FROM game_item_container_slots pack JOIN carried_key k USING (world_id)
         WHERE pack.character_id=encode($5,'hex')::uuid",
    )
    .bind(id(42).as_slice())
    .bind(&reward.item)
    .bind(i64::from(reward.count))
    .bind(i32::from(reward.key_number.ok_or("Prisoner's key number")?))
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(seeded.rows_affected(), 1);
    tx.commit().await?;

    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", &reward.item, "definition-r1"))
        .ok_or("native key definition")?;
    for (revision, blocked) in [(0, false), (1, true), (2, false)] {
        let door = object_target(owner, position, 1)?;
        assert_eq!(door.expected_revision, revision);
        if revision == 2 {
            let refusal = use_target(owner, admitted, *command, &door).await?;
            *command += 1;
            assert_eq!(
                progress(h, "passage").await?,
                -1,
                "a relocked door cannot write quest progress"
            );
            assert_eq!(refusal, UseDisposition::Locked);
        }
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(door)),
            use_with_position: None,
        })
        .map_err(debug)?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let (next, disposition, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            *command,
            &payload,
            Some(reference),
        )
        .await?;
        *admitted = next;
        *command += 1;
        assert_eq!(disposition, UseDisposition::Committed);
        let facts = &owner.bundle_map().ok_or("numbered door map")?.facts;
        assert_eq!(facts.doors().blocks(position), blocked);
    }
    Ok(())
}

async fn clear_group(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    count: usize,
    command: u64,
) -> TestResult {
    let actors = owner.quest_guardians_for_test(key).await;
    assert_eq!(actors.len(), count);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        owner.quest_guardians_for_test(key).await,
        actors,
        "refresh cannot duplicate group slots"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
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
    for (index, actor) in actors.into_iter().enumerate() {
        super::grizzly_npc_fixtures::questpc8_campaign::lethal(
            owner,
            admitted,
            actor,
            "oteryn:creature.demon",
            command,
            u16::try_from(index + 1)?,
        )
        .await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(owner.quest_guardians_for_test(key).await.is_empty());
    Ok(())
}

pub(super) async fn raise_level(
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

async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/magic_sword/{suffix}")).fetch_one(&h.pool).await?)
}

async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    pos: TilePos,
) -> TestResult<UseDisposition> {
    let target = object_target(owner, pos, 1)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::foundation::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..64 {
        let at = position(owner, admitted).await?;
        let direction = if at.x < x {
            StepDirection::East
        } else if at.x > x {
            StepDirection::West
        } else if at.y < y {
            StepDirection::South
        } else if at.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, *command, direction).await?;
        *command += 1;
    }
    Err("route did not arrive".into())
}
