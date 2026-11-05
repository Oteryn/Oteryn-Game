//! Canary b14adcfd: a death/respawn/reconnect must not revive container handles.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::container_view::{
    ContainerObservation, ContainerPlan, ContainerViewState,
};
use crate::gameplay_transport::item_view::{
    InventoryItems, ItemKey, ItemViewContinuity, SessionItemView, ViewItem,
};
use oteryn_protocol_oteryn::container_tree::{
    ContainerViewIntent, ContainerViewOutcome, decode_container_views,
};
use std::num::NonZeroU32;

#[test]
fn donor_f27_container_handle_refused_after_death_respawn_reconnect() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "donor_f27", true, false).await?;
            prepare_current_session(&h).await?;
            super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 5)]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let current = crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_lease_generation: 2, ..fence(1)?
            };
            // Independent external Ground backpack; historical fixture DATA.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,encode($2,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($3,'hex')::uuid")
                .bind(id(80).as_slice()).bind(id(82).as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'\\x01020307'::bytea,'external','map-1','content-1','entry',800)")
                .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,encode($2,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($3,'hex')::uuid")
                .bind(id(81).as_slice()).bind(id(83).as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
                .bind(id(81).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(80).as_slice()).bind(id(84).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let bag = ViewItem { key: ItemKey::Instance(id(80)), item_definition_ref: NonZeroU32::MIN, count: NonZeroU32::MIN, sub_type: 0 };
            let child = ViewItem { key: ItemKey::Instance(id(81)), count: NonZeroU32::new(5).ok_or("child count")?, ..bag };
            let inventory = InventoryItems::default;
            let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_container_tree().with_map_view();
            view.snapshot(inventory(), None).map_err(debug)?;
            view.views_snapshot().map_err(debug)?;
            view.map_view(&[bag.key], |_| Ok(())).map_err(debug)?;
            let mut containers = ContainerViewState::default();
            let handle = view.table().handle(&bag.key).ok_or("bag handle")?;
            let intent = |handle| ContainerViewIntent::Open { handle, replace_view: None };
            let ContainerPlan::Observe(pending) = containers.plan(&mut view, intent(handle)).map_err(debug)? else { return Err("bag observation required".into()); };
            let (outcome, delta) = containers.apply(&mut view, pending, Some(ContainerObservation::Container { parent: None, capacity: 20, entries: vec![child] })).map_err(debug)?;
            assert_eq!(outcome, ContainerViewOutcome::Opened);
            let shown = decode_container_views(&delta.ok_or("open delta")?.payload).map_err(debug)?;
            let old_child = shown.views[0].entries[0].handle;
            assert_eq!(view.resolve(old_child), Some(child.key));
            let request = crate::gameplay_transport::death_ml_loss_postgres::death(90)?;
            let death = owner.settle_source_player_death(current, request, None, None).await.map_err(debug)?;
            h.root.consume_pending_respawn(&authority, &h.node,
                crate::durability::character_progression::CurrentCharacterGameplayFence { expected_character_revision: death.committed_character_revision, ..current },
                Some(death.occurrence)).await.map_err(debug)?.ok_or("durable respawn")?;
            // Production reconnect construction: retain counters, rebuild handle tables and
            // start with no open views. No death flag is fabricated in the projection.
            let continuity = view.continuity();
            let mut resumed = SessionItemView::resume(continuity).with_container_tree().with_map_view();
            let mut containers = ContainerViewState::default();
            resumed.snapshot(InventoryItems::default(), None).map_err(debug)?;
            resumed.views_snapshot().map_err(debug)?;
            for old in [handle, old_child] {
                assert!(resumed.resolve(old).is_none());
                assert!(matches!(containers.plan(&mut resumed, intent(old)).map_err(debug)?, ContainerPlan::Decided(ContainerViewOutcome::Stale, None)));
            }
            // A fresh Ground observation is required even for the same IDs.
            let stored = h.root.read_character_backpack(&authority, current.character_id).await.map_err(debug)?.ok_or("preserved novice backpack")?;
            assert_eq!(stored.backpack.item_instance_id, id(70));
            let observed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(id(80).as_slice()).fetch_one(&h.pool).await?;
            assert!(observed, "fresh external Ground observation");
            resumed.map_view(&[bag.key], |_| Ok(())).map_err(debug)?;
            let fresh = resumed.table().handle(&bag.key).ok_or("fresh bag handle")?;
            assert!(fresh.get() > handle.get());
            let ContainerPlan::Observe(pending) = containers.plan(&mut resumed, intent(fresh)).map_err(debug)? else { return Err("fresh root must observe contents".into()); };
            let (outcome, delta) = containers.apply(&mut resumed, pending, Some(ContainerObservation::Container { parent: None, capacity: 20, entries: vec![child] })).map_err(debug)?;
            assert_eq!(outcome, ContainerViewOutcome::Opened);
            let shown = decode_container_views(&delta.ok_or("fresh open delta")?.payload).map_err(debug)?;
            let fresh_child = shown.views[0].entries[0].handle;
            assert!(fresh_child.get() > old_child.get());
            assert_eq!(resumed.resolve(fresh_child), Some(child.key));
            assert!(resumed.resolve(old_child).is_none());
            assert!(resumed.resolve(handle).is_none());
            eprintln!("POSTGRES_STARTED donor_f27: death receipt and respawn consumed on PostgreSQL 18.6");
            drop(owner); drop(authority); drop(seal); h.cleanup().await
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "F27 thread panicked")?.map_err(Into::into)
}
