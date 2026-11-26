use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::{
    connection::FreshAdmissionAuthority,
    container_view::{ContainerPlan, ContainerViewState},
    item_fence_of,
    item_view::{ItemKey, ItemViewContinuity, SessionItemView},
};
use oteryn_protocol_oteryn::container_tree::{ContainerViewIntent, ContainerViewOutcome};
use oteryn_protocol_oteryn::item_look::ItemLookResult;

#[test]
fn carried_document_look_includes_description_and_short_durable_text() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "PostgreSQL route must be nonempty");
    super::item_look_fixtures::large_stack(|| {
        run(|admin| async {
            let harness =
                Harness::create_with_admission_seed(admin, "carried_document_look", true, false)
                    .await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let (controller, pin) = activated_with_loot_tables(deps.admitted.world_id, None);
            let active = controller.active().ok_or("active generation")?;
            let content_pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (6, 0, 7),
            );
            let deps = Dependencies::with_content(content_pin, deps.room, deps.spells)?;
            deps.runtime
                .lock()
                .await
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 6,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(active);
            deps.initialize(&owner).await;
            let session = deps.admitted.game_session_id;
            let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
                .current_session_at(session)
                .await?;
            let fence = item_fence_of(current).ok_or("item fence")?;

            const DOCUMENT: &str = "oteryn:item.tibia.i2818";
            const BACKPACK: &str = "oteryn:item.tibia.i2854";
            const SHORT_TEXT: &str = "Meet by the bridge. Bring a torch. 🐢";
            super::item_look_fixtures::seed_main_backpack(
                &harness,
                &[(DOCUMENT, 1), (DOCUMENT, 1), (BACKPACK, 1), (DOCUMENT, 1)],
            )
            .await?;
            let mut tx = harness.pool.begin().await?;
            // Seed item 74 under the inner backpack so only its container view grants a handle.
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "WITH entry AS (
                     DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid
                     RETURNING item_instance_id, world_id, placement_ordinal, placed_transaction_id
                 )
                 INSERT INTO game_item_nested_entries
                     (item_instance_id, world_id, parent_item_instance_id, placement_ordinal, placed_transaction_id)
                 SELECT item_instance_id, world_id, encode($2,'hex')::uuid, placement_ordinal, placed_transaction_id
                 FROM entry",
            )
            .bind(id(74).as_slice())
            .bind(id(73).as_slice())
            .execute(&mut *tx)
            .await?;
            for item in [id(71), id(74)] {
                sqlx::query(
                    "INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)",
                )
                .bind(item.as_slice())
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO game_item_texts(item_instance_id,text,written_at,text_revision)
                     VALUES(encode($1,'hex')::uuid,$2,now(),1)",
                )
                .bind(item.as_slice())
                .bind(SHORT_TEXT)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            let inventory = owner
                .observe_character_inventory(deps.actor, session)
                .await
                .ok_or("current inventory")?;
            let mut views = SessionItemView::resume(ItemViewContinuity::default());
            views.snapshot(inventory, None).map_err(debug)?;
            let inner = ItemKey::Instance(id(73));
            let nested = ItemKey::Instance(id(74));
            assert_eq!(views.table().handle(&nested), None);
            let handle = views
                .table()
                .handle(&inner)
                .ok_or("inner backpack handle")?;
            let mut containers = ContainerViewState::default();
            let ContainerPlan::Observe(pending) = containers
                .plan(
                    &mut views,
                    ContainerViewIntent::Open {
                        handle,
                        replace_view: None,
                    },
                )
                .map_err(debug)?
            else {
                return Err("inner backpack open did not reach the owner".into());
            };
            let contents = owner
                .observe_container(deps.actor, Some(fence), inner)
                .await;
            let (opened, _) = containers
                .apply(&mut views, pending, contents)
                .map_err(debug)?;
            assert_eq!(opened, ContainerViewOutcome::Opened);
            let look = async |item| -> TestResult<ItemLookResult> {
                let handle = views
                    .table()
                    .handle(&ItemKey::Instance(item))
                    .ok_or("item handle")?;
                Ok(owner
                    .look_item(deps.actor, session, Some(fence), &views, handle)
                    .await)
            };
            eprintln!(
                "I03SRV PostgreSQL fixture ready: carried_document_look_includes_description_and_short_durable_text"
            );
            let ItemLookResult::Text(blank) = look(id(72)).await? else {
                return Err("blank document LOOK refused".into());
            };
            assert!(blank.contains("\nIt is rewriteable."), "{blank}");
            assert!(!blank.contains("You read:"), "{blank}");
            let expected = ItemLookResult::Text(format!("{blank}\nYou read: {SHORT_TEXT}"));
            assert_eq!(look(id(71)).await?, expected, "inventory document");
            assert_eq!(look(id(74)).await?, expected, "nested container document");
            let backpack = active
                .native_gameplay()
                .and_then(|native| native.item_policy(BACKPACK, "definition-r1"))
                .ok_or("backpack policy")?;
            let ordinary = oteryn_gameplay_items::look::item_text(
                &backpack.record().semantics,
                std::num::NonZeroU32::MIN,
                -1,
            )
            .ok_or("backpack description")?;
            assert_eq!(look(id(70)).await?, ItemLookResult::Text(ordinary));
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            Ok(())
        })
    })
}
