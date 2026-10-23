//! ITEM-TIMED-1: admitted session -> real framed dispatch -> durable item/fee -> reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MessageType};
use crate::gameplay_transport::item_view::{InventoryItems, ItemKey};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::*};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::timed_item::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn intent_bytes(value: TimedItemIntent) -> TestResult<Vec<u8>> {
    encode_timed_item_intent(value).map_err(debug)
}
fn result_value(bytes: &[u8]) -> TestResult<TimedItemResult> {
    decode_timed_item_result(bytes).map_err(debug)
}

#[path = "timed_item/authority.rs"]
mod authority;
#[path = "timed_item/duration.rs"]
mod duration;
#[path = "timed_item/recovery.rs"]
mod recovery;
#[path = "timed_item/repair.rs"]
mod repair;
#[path = "timed_item/standard_use.rs"]
mod standard_use;

struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn observe_character_inventory(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<InventoryItems> {
        self.0.observe_character_inventory(actor, session).await
    }
    async fn observe_character_equipment(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<
        Vec<(
            oteryn_protocol_oteryn::item_view::EquipmentSlot,
            crate::gameplay_transport::item_view::ViewItem,
        )>,
    > {
        self.0.observe_character_equipment(actor, session).await
    }
    async fn committed_equipment_move(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
    ) -> Option<crate::gameplay_transport::item_move::EquipmentMoveReplay> {
        self.0.committed_equipment_move(actor, command).await
    }
    async fn move_equipment(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        request: crate::gameplay_transport::item_move::EquipmentMove,
    ) -> crate::gameplay_transport::item_move::EquipmentMoveResult {
        self.0.move_equipment(actor, command, request).await
    }
    async fn use_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> UseOutcome {
        self.0.use_item(actor, command, item).await
    }
    async fn use_item_completion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<UseOutcome> {
        self.0.use_item_completion(actor, command, item).await
    }
    async fn timed_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        payload: &[u8],
        target: Option<ItemKey>,
    ) -> Option<Vec<u8>> {
        self.0.timed_item(actor, command, payload, target).await
    }
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    commands: &[(u64, Vec<u8>)],
) -> TestResult<Vec<Vec<u8>>> {
    exchange_kind(owner, admitted, COMMAND_TYPE_TIMED_ITEM_INTENT, commands).await
}
async fn exchange_kind(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    kind: u32,
    commands: &[(u64, Vec<u8>)],
) -> TestResult<Vec<Vec<u8>>> {
    admitted.continuity.next_command_id = commands.first().map_or(1, |(id, _)| *id);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (id, payload) in commands {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: *id,
                command_type: kind,
                payload,
            },
        )?;
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, &WireOwner(owner), IDLE_LIVENESS).await;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < bytes.len() {
        let len = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
        let frame = crate::foundation::decode_wire_envelope(&bytes[cursor + 4..cursor + 4 + len])?;
        if frame.message_type() == MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            results.push(result.payload.to_vec());
        }
        cursor += 4 + len;
    }
    Ok(results)
}

#[test]
fn timed_item_postgres_wire_charges_replay_repair_and_owner_expiry() -> TestResult {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "timed item thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "timed_item_wire", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        assert!(
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(
                true,
                Some(active)
            )
            .iter()
            .any(|offer| offer.id == CAPABILITY_TIMED_ITEM_V1)
        );
        assert!(
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(true, None)
                .iter()
                .all(|offer| offer.id != CAPABILITY_TIMED_ITEM_V1)
        );
        let index = native.item_index().ok_or("index")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("current")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (seed, key, quantity) in [
            (70, "oteryn:item.tibia.i2854", 1_i64),
            (80, "oteryn:item.tibia.i6530", 1),
            (90, "oteryn:item.tibia.i6529", 1),
            (100, "oteryn:item.tibia.i3081", 1),
            (110, "oteryn:item.tibia.i3043", 2),
        ] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,encode($6,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or("item revision")?).bind(quantity).bind(id(seed+1).as_slice()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
        for (seed, ordinal) in [(80, 1_i64), (90, 2), (100, 3), (110, 4)] {
            sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(ordinal.to_string()).bind(id(seed+2).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

        let selected = crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, CAPABILITY_TIMED_ITEM_V1],
        )
        .ok_or("selection")?;
        let admitted = AdmittedSession {
            item_fence: Some(fence.item_fence()),
            first_entry: FirstEntryOutcome::Positioned,
            continuity: SessionContinuity {
                selected_capabilities: selected,
                ..SessionContinuity::FRESH
            },
            ..deps.admitted
        };
        authority::exercise(&owner, &h, admitted).await?;
        // Inventory order is B3 (newest first): main=1, crystal=2, amulet=3, boots=4, worn=5.
        let intent = TimedItemIntent {
            action: TimedAction::Use,
            target: ItemHandle::new(3).ok_or("handle")?,
            expected_revision: 0,
            npc: None,
        };
        let payload = intent_bytes(intent)?;
        let results = exchange(&owner, admitted, &[(1, payload.clone())]).await?;
        assert_eq!(results.len(), 1, "real wire result");
        let result = result_value(&results[0])?;
        assert_eq!(result.disposition, TimedDisposition::Ok);
        assert_eq!(result.charges, Some(4));
        assert_eq!(result.revision, 1);
        // Reconstruct the runtime owner, retaining only durable state and the session.
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let replay = exchange(&owner, admitted, &[(1, payload)]).await?;
        assert_eq!(
            replay, results,
            "committed command replays after owner restart"
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_state_writes")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1);
        let mut without = admitted;
        without.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::NONE;
        assert_eq!(
            exchange(&owner, without, &[(1, intent_bytes(intent)?)]).await?,
            [Vec::<u8>::new()]
        );
        // Stale expected row cannot spend another charge.
        let stale = exchange(&owner, admitted, &[(2, intent_bytes(intent)?)]).await?;
        assert_eq!(
            result_value(&stale[0])?.disposition,
            TimedDisposition::Stale
        );
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let inspected = exchange(
            &owner,
            admitted,
            &[(
                3,
                intent_bytes(TimedItemIntent {
                    action: TimedAction::Inspect,
                    ..intent
                })?,
            )],
        )
        .await?;
        assert_eq!(result_value(&inspected[0])?.charges, Some(4));
        assert_eq!(result_value(&inspected[0])?.revision, 1);
        // Exhaust the remaining charges through command bytes, including the final durable burn.
        let commands = (1..=4)
            .map(|revision| {
                Ok((
                    revision + 3,
                    intent_bytes(TimedItemIntent {
                        expected_revision: revision,
                        ..intent
                    })?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        let spent = exchange(&owner, admitted, &commands).await?;
        assert_eq!(spent.len(), 4);
        assert!(result_value(&spent[3])?.retired);
        let lifecycle:i16=sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(100).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lifecycle, 2);
        recovery::exercise(&owner, &h, admitted, &commands[3].1).await?;
        repair::exercise(&mut owner, &h, &deps, admitted).await?;
        duration::exercise(&mut owner, &h, &deps, admitted).await?;
        standard_use::exercise(&owner, &h, admitted).await?;
        authority::concurrency(&owner, &h, admitted)
            .await
            .map_err(|error| format!("timed concurrency: {error}"))?;
        drop(owner);
        drop(authority);
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("restart authority: {error:?}"))?;
        let repaired = h
            .root
            .read_timed_row(&reopened, &h.node, fence, id(80))
            .await
            .map_err(debug)?
            .ok_or("repaired reload")?;
        assert_eq!(
            repaired.values.and_then(|values| values.remaining_ms()),
            Some(14_400_000)
        );
        drop(reopened);
        drop(seal);
        h.cleanup().await
    })
}
