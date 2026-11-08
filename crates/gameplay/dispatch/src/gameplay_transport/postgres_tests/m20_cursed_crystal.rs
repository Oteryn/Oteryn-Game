//! Pinned donor self-use through the production carried quest item caller.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn m20_molten_wax_self_use_consumes_one_and_replay_is_idempotent_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M20 wax thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED m20_molten_wax duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

fn scenario(recipe: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "m20_molten_wax", true, false).await?;
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::from_activation(
                deps.admitted.world_id,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (31942, 32936, 10),
            ),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 31942,
                    y: 32936,
                    floor: 10,
                },
            )
            .map_err(debug)?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        let player = runtime
            .visible_entities()
            .players
            .into_iter()
            .find(|p| p.actor == deps.actor)
            .ok_or("visible player")?;
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: deps.actor.placement_identity(),
            generation: player.generation,
        };
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        if recipe {
            return inventory_recipe(&owner, &deps, &h, fence).await;
        }
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i9106", 1),
                ("oteryn:item.tibia.i9106", 1),
            ],
        )
        .await?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("handle")?),
            use_with: Some(target),
            use_with_map_item: None,
            use_with_position: None,
        };
        let command = crate::gameplay_transport::connection::UseCommand {
            game_session_id: fence.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let source = Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
            71,
        )));
        let mut wrong = intent.clone();
        wrong.use_with.as_mut().ok_or("target")?.generation += 1;
        let refused = owner
            .use_event_item(deps.actor, command, wrong, source, true)
            .await
            .map_err(|_| "wrong self-target route")?
            .ok_or("wrong target outcome")?;
        assert_eq!(refused.disposition, UseDisposition::Rejected);
        assert_eq!(quantity(&h).await?, 2);
        let committed = owner
            .use_event_item(deps.actor, command, intent.clone(), source, true)
            .await
            .map_err(|_| "wax route")?
            .ok_or("wax outcome")?;
        assert_eq!(committed.disposition, UseDisposition::Committed);
        assert_eq!(quantity(&h).await?, 1);
        let replay = owner
            .use_event_item(deps.actor, command, intent.clone(), source, true)
            .await
            .map_err(|_| "wax replay route")?
            .ok_or("wax replay outcome")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert_eq!(quantity(&h).await?, 1);
        let mut next_intent = intent;
        next_intent.target =
            UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("second wax handle")?);
        let again = owner
            .use_event_item(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    command_id: 2,
                    ..command
                },
                next_intent,
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                    72,
                ))),
                true,
            )
            .await
            .map_err(|_| "wax second command")?
            .ok_or("wax second outcome")?;
        assert_eq!(again.disposition, UseDisposition::Rejected);
        assert_eq!(quantity(&h).await?, 1);
        let stage: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind("oteryn:quest-progress/quest/u10_70/the_cursed_crystal/oneeyedjoe").fetch_one(&h.pool).await?;
        assert_eq!(stage, 2);
        Ok(())
    })
}
async fn quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9106' AND lifecycle=1")
        .fetch_one(&h.pool).await?)
}

#[test]
fn m20_existing_craft_receipt_atomic_debit_grant_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M20 recipe thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED m20_first_ointment duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

async fn inventory_recipe(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i11466", 1),
            ("oteryn:item.tibia.i21504", 1),
        ],
    )
    .await?;
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("source")?),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
            oteryn_protocol_oteryn::item_view::ItemHandle::new(3).ok_or("target")?,
        )),
    };
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: fence.game_session_id,
        command_id: 1,
        item_fence: Some(fence),
    };
    let source = Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
        71,
    )));
    let target = Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
        72,
    )));
    // A fixture with the target outside custody must leave both snapshots unchanged.
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(72).as_slice())
    .execute(&mut *setup)
    .await?;
    setup.commit().await?;
    let before: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM game_item_instances i ORDER BY item_instance_id",
    )
    .fetch_all(&h.pool)
    .await?;
    let refused = owner
        .use_craft(deps.actor, command, intent.clone(), source, target)
        .await;
    assert!(refused.is_none_or(|outcome| outcome.disposition != UseDisposition::Committed));
    let after: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM game_item_instances i ORDER BY item_instance_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(before, after);
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).execute(&mut *setup).await?;
    setup.commit().await?;
    let committed = owner
        .use_craft(deps.actor, command, intent.clone(), source, target)
        .await
        .ok_or("recipe outcome")?;
    assert_eq!(committed.disposition, UseDisposition::Committed);
    let replay = owner
        .use_craft(deps.actor, command, intent, source, target)
        .await
        .ok_or("recipe replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    let items:Vec<(String,i64)> = sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i11466','oteryn:item.tibia.i21504','oteryn:item.tibia.i21505') ORDER BY definition_production_key").fetch_all(&h.pool).await?;
    assert_eq!(items, vec![("oteryn:item.tibia.i21505".into(), 1)]);
    let receipt: (i64, i64) = sqlx::query_as(
        "SELECT count(*), COALESCE(sum(ingredient_count),0)::bigint FROM game_item_craft_records",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipt, (1, 1));
    Ok(())
}
