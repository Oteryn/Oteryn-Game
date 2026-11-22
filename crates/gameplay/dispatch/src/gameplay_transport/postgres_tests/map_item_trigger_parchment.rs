//! Native parchment custody and the four-spawn encounter through item-move dispatch.
use super::fixture::{Harness, debug, id};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::{
    connection::{FirstEntryOutcome, FreshAdmissionAuthority, UseCommand},
    item_move::{LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
};
use oteryn_gameplay_items::map_pickup::PARCHMENT_SEAL_PLACEMENT;
use oteryn_gameplay_world::map::view::MapFacts;

#[test]
fn parchment_removal_starts_four_demons_once_and_schedules_the_seal_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "parchment_removal", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("content generation")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let origin = crate::foundation::MovementLocalPosition {
            x: 33063,
            y: 31625,
            floor: 15,
        };
        let world = native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x129-y123.b3"
                )),
                region: (15, 129, 123),
                bounds: (33059, 31622, 33068, 31629),
                start: (33063, 31625),
                expected: vec![ExpectedTile {
                    position: (33063, 31624),
                    ids: vec![353, 2474, 2819],
                    counted_entry: None,
                }],
            },
        )?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
            )
            .ok_or("tomb bundle room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: admitted.item_fence,
        };
        let request = LoopMove {
            item: ItemKey::MapBase {
                bundle_digest: world.base().digest(),
                placement_key: PARCHMENT_SEAL_PLACEMENT,
                reset_epoch: 0,
            },
            target: LoopTarget::MainBackpack,
            count: None,
            slot: None,
        };
        let text = world
            .facts()
            .base_text(PARCHMENT_SEAL_PLACEMENT)
            .ok_or("seal inscription")?;
        assert_eq!(
            owner.move_loop_item(deps.actor, command, request).await,
            LoopMoveResult::Committed
        );
        assert_eq!(
            owner.committed_loop_move(deps.actor, command).await,
            Some(LoopMoveReplay::Moved)
        );
        assert_eq!(
            owner
                .move_loop_item(
                    deps.actor,
                    UseCommand {
                        command_id: 2,
                        ..command
                    },
                    request
                )
                .await,
            LoopMoveResult::Refused(LoopMoveRefusal::Stale)
        );
        let native = owner.runtime.lock().await;
        let mut positions = native
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .map(|actor| {
                native
                    .read_actor_position(actor)
                    .map(|snapshot| {
                        let at = snapshot.position();
                        (at.x, at.y, at.floor)
                    })
                    .map_err(debug)
            })
            .collect::<Result<Vec<_>, _>>()?;
        positions.sort_unstable();
        assert_eq!(
            positions,
            [
                (33060, 31623, 15),
                (33060, 31627, 15),
                (33066, 31623, 15),
                (33066, 31627, 15)
            ]
        );
        drop(native);
        let (has_deadline, inscription): (bool, String) = sqlx::query_as(
            "SELECT p.reset_at BETWEEN r.created_at + interval '2 hours' AND r.created_at + interval '2 hours 1 second', t.text \
             FROM game_parchment_removals p JOIN game_item_loop_move_receipts r ON r.transaction_id=p.move_transaction_id \
             JOIN game_map_item_mint_receipts m ON m.transaction_id=p.mint_transaction_id \
             JOIN game_item_texts t ON t.item_instance_id=m.item_instance_id",
        ).fetch_one(&h.pool).await?;
        assert!(has_deadline);
        assert_eq!(inscription, text);
        // Advance the persisted deadline in this disposable database. The next
        // move still goes through the live reset admission and the normal MINT.
        let mut tx = h.pool.begin().await?;
        sqlx::query(
            "ALTER TABLE game_parchment_removals DISABLE TRIGGER game_parchment_removal_immutable",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE game_parchment_removals SET reset_at=clock_timestamp()-interval '1 second'",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "ALTER TABLE game_parchment_removals ENABLE TRIGGER game_parchment_removal_immutable",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        owner.states.loop_ground.invalidate();
        assert_eq!(
            owner
                .move_loop_item(
                    deps.actor,
                    UseCommand {
                        command_id: 3,
                        ..command
                    },
                    request
                )
                .await,
            LoopMoveResult::Committed
        );
        let (mints, linked): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(parchment_previous) FROM game_map_item_mint_receipts WHERE placement_key=$1::text::numeric",
        ).bind(PARCHMENT_SEAL_PLACEMENT.to_string()).fetch_one(&h.pool).await?;
        assert_eq!((mints, linked), (2, 1));

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
