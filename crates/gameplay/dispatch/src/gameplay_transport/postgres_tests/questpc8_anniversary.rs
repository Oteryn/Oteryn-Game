//! Chosen seasonal route: real native deaths, actual corpse collection and exact owned input.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[path = "questpc8_anniversary/world.rs"]
mod fixture_world;

#[test]
fn questpc8_anniversary_thirty_native_rag_drops_five_pinatas_and_once_only_backpack() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-anniversary", || {
        quest_world_callbacks::scenario(
            "authored-anniversary",
            Position {
                x: 32359,
                y: 32234,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_anniversary_thirty_native_rag_drops_five_pinatas_and_once_only_backpack duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    fixture_world::world(active.native_gameplay().ok_or("native items")?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mut command = 1;
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
    super::grizzly_npc::questpc8_campaign::hunt(owner, admitted, "oteryn:creature.wolf", 1, 5000)
        .await?;
    assert_eq!(progress(h, "stage").await?, -1);
    let premature:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i24415'").fetch_one(&h.pool).await?;
    assert_eq!(
        premature, 0,
        "an actual native wolf death before accepting the programme cannot drop a quest rag"
    );
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 1);
    let table = TilePos {
        x: 32359,
        y: 32235,
        floor: -7,
    };
    assert_eq!(
        claim(owner, admitted, &mut command, table).await?,
        UseDisposition::Rejected
    );
    assert_eq!(quantity(h, 30197).await?, 0);
    for index in 0..30_u16 {
        let animal = ["cyclops", "wolf", "troll", "bear"][usize::from(index) % 4];
        let key = format!("oteryn:quest-death/authored-anniversary-{animal}");
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let actors = owner.quest_guardians_for_test(&key).await;
        assert_eq!(actors.len(), 1, "one active native {animal} guardian");
        let target = actors[0];
        super::grizzly_npc::questpc8_campaign::lethal(
            owner,
            admitted,
            target,
            &format!("oteryn:creature.{animal}"),
            command,
            index + 1,
        )
        .await?;
        assert_eq!(progress(h, "rags").await?, i64::from(index + 1));
        collect_rag(owner, admitted, h, u64::from(index)).await?;
        assert_eq!(
            quantity(h, 24415).await?,
            i64::from(index + 1),
            "every unit came through a real corpse MINT and transfer"
        );
        assert_eq!(quantity(h, 30197).await?, 0);
        {
            let mut runtime = owner.runtime.lock().await;
            assert!(!runtime.corpse_projection_pinned(target));
            runtime.remove_encounter_actor(target).map_err(debug)?;
        }
        if index == 28 {
            assert_eq!(
                claim(owner, admitted, &mut command, table).await?,
                UseDisposition::Rejected,
                "twenty-nine units cannot satisfy the exact thirty-unit hand-in"
            );
            assert_eq!(quantity(h, 24415).await?, 29);
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(61))
            .ok_or("clock")?;
    }
    let transferred:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i24415' AND r.loot_purpose_key='oteryn:item.tibia.i24415'").fetch_one(&h.pool).await?;
    assert_eq!(
        transferred, 30,
        "distinct native death causes minted thirty rags"
    );
    let target = object_target(owner, table, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(progress(h, "stage").await?, 2);
    assert_eq!(
        quantity(h, 24415).await?,
        0,
        "all thirty owned rags consumed once"
    );
    assert_eq!(
        claim(owner, admitted, &mut command, table).await?,
        UseDisposition::Rejected
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let dragons = owner
        .quest_guardians_for_test("oteryn:quest-death/authored-anniversary-pinata")
        .await;
    assert_eq!(dragons.len(), 5);
    for (index, dragon) in dragons.into_iter().enumerate() {
        super::grizzly_npc::questpc8_campaign::lethal(
            owner,
            admitted,
            dragon,
            "oteryn:creature.pinata_dragon",
            command,
            u16::try_from(index + 40)?,
        )
        .await?;
        assert_eq!(progress(h, "kills").await?, i64::try_from(index + 1)?);
        if index == 3 {
            assert_eq!(
                claim(owner, admitted, &mut command, table).await?,
                UseDisposition::Rejected
            );
        }
    }
    let target = object_target(owner, table, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(quantity(h, 30197).await?, 1);
    assert_eq!(progress(h, "stage").await?, 3);
    assert_eq!(
        claim(owner, admitted, &mut command, table).await?,
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
        claim(owner, admitted, &mut command, table).await?,
        UseDisposition::Rejected
    );
    assert_eq!(quantity(h, 30197).await?, 1);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.tibia_anniversary'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn collect_rag(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    ordinal: u64,
) -> TestResult {
    use crate::durability::{
        item_mint::TypedDefinitionRef,
        item_transfer::{ItemTransferDestination, ItemTransferRequest},
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let raw:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE i.definition_production_key='oteryn:item.tibia.i24415' AND i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT 1").fetch_one(&h.pool).await?;
    let item: [u8; 16] = raw.try_into().map_err(|_| "item UUID")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    let reference = |id| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let mut candidate = owner
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(3000 + ordinal).map_err(debug)?,
                ),
                source_item_instance_id: item,
                destination: ItemTransferDestination::MainBackpack,
                item: generation_item_facts(native, &reference(24415)).map_err(debug)?,
                backpack: Some(generation_item_facts(native, &reference(2854)).map_err(debug)?),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    owner
        .root
        .commit_item_transfer(
            &permit,
            owner.character,
            owner.holder,
            fence,
            &mut candidate,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    Ok(())
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/anniversary/{suffix}")).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
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
