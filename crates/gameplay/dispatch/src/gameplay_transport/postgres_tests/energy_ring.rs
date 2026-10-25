//! ENERGY-RING-1: framed equipment dispatch, real damage owner and PostgreSQL expiry.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MessageType};
use crate::gameplay_transport::item_view::{InventoryItems, ItemKey};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::*};
use oteryn_protocol_oteryn::item_view::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn energy_ring_postgres_dispatch_equip_damage_pause_and_expiry() -> TestResult {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "energy ring thread")??;
    Ok(())
}

async fn move_ring(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    handle: u64,
    destination: ItemMoveDestination,
) -> TestResult {
    let bytes = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: ItemHandle::new(handle).ok_or("ring handle")?,
            destination,
        },
        ItemMoveSelection {
            equip_drop: true,
            container_tree: false,
            world_interaction: false,
        },
    )
    .map_err(debug)?;
    let result = exchange_kind(
        owner,
        admitted,
        COMMAND_TYPE_ITEM_MOVE_INTENT,
        &[(command, bytes)],
    )
    .await?;
    assert_eq!(result.len(), 1);
    assert_eq!(
        decode_item_move_result(&result[0]).map_err(debug)?,
        ItemMoveOutcome::Moved
    );
    Ok(())
}

async fn hit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    damage: u32,
) -> TestResult<(u32, u32)> {
    use crate::ability::creature_bite::CreatureBiteVitals;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    states
        .apply_creature_damage(
            &mut runtime,
            actor,
            session,
            damage,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                owner.owner_now().get(),
            ),
        )
        .ok_or("damage owner")?;
    let state = states.get(&runtime, actor, session).ok_or("state")?;
    Ok((state.vitals().health, state.vitals().mana))
}

async fn budget(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "energy_ring", true, false).await?;
        eprintln!("POSTGRES_STARTED ENERGY-RING-1");
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
        for (seed, key) in [
            (70, "oteryn:item.tibia.i2854"),
            (90, "oteryn:item.tibia.i3051"),
        ] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or("revision")?).bind(id(seed+1).as_slice()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(92).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',10,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='knight'")
            .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .initialize(
                    &runtime,
                    deps.actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        level: 50,
                        magic_level: 10,
                        max_health: 1000,
                        max_mana: 101,
                        max_soul: 100,
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("initialize")?;
        }
        let admitted = AdmittedSession {
            item_fence: Some(fence.item_fence()),
            first_entry: FirstEntryOutcome::Positioned,
            continuity: SessionContinuity {
                // The gameplay rule and timer do not depend on the optional timed-item UI.
                selected_capabilities:
                    crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[4, 6, 12],
                    )
                    .ok_or("capabilities")?,
                ..SessionContinuity::FRESH
            },
            ..deps.admitted
        };
        assert_eq!(
            hit(&owner, deps.actor, session, 5).await?,
            (995, 101),
            "backpack grants no shield"
        );
        move_ring(
            &owner,
            admitted,
            1,
            2,
            ItemMoveDestination::Equipment(EquipmentSlot::Ring),
        )
        .await?;
        let view =
            crate::gameplay_transport::equipment_move::observe_character_equipment(&owner, session)
                .await
                .ok_or("worn view")?;
        assert_eq!(view.len(), 1);
        let active_ref = crate::gameplay_transport::item_ref_admission::definition_ref(
            index,
            &crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i3088".into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .ok_or("active reference")?;
        assert_eq!(view[0].1.item_definition_ref, active_ref);
        // Admission reconstructs the equipment contribution from durable custody too.
        {
            let runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .get_mut(&runtime, deps.actor, session)
                .ok_or("state before admission reload")?
                .install_equipped_mana_shield(false);
        }
        let effects = owner
            .read_admission_effects(session)
            .await
            .map_err(|_| "admission effects")?;
        {
            let runtime = owner.runtime.lock().await;
            owner
                .install_admission_suppressions(&runtime, deps.actor, session, effects)
                .await
                .ok_or("admission install")?;
        }
        let mut wrong_slot =
            crate::gameplay_transport::equipment_move::read_current_equipment(&owner, session)
                .await
                .ok_or("worn snapshot")?;
        wrong_slot.items[0].slot = 4;
        assert_eq!(crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_mana_shield(&wrong_slot, native), Some(false));
        assert_eq!(hit(&owner, deps.actor, session, 5).await?, (995, 91));
        owner.clock_origin -= std::time::Duration::from_millis(60_001);
        owner.tick_timed_items().await;
        assert!((530_000..540_000).contains(&budget(&h).await?));
        move_ring(&owner, admitted, 2, 2, ItemMoveDestination::MainBackpack).await?;
        let paused = budget(&h).await?;
        owner.clock_origin -= std::time::Duration::from_secs(100);
        owner.tick_timed_items().await;
        assert_eq!(budget(&h).await?, paused, "backpack time is paused");
        assert_eq!(hit(&owner, deps.actor, session, 5).await?, (990, 91));
        move_ring(
            &owner,
            admitted,
            3,
            2,
            ItemMoveDestination::Equipment(EquipmentSlot::Ring),
        )
        .await?;
        assert_eq!(hit(&owner, deps.actor, session, 5).await?, (990, 81));
        owner.clock_origin -= std::time::Duration::from_secs(600);
        owner.tick_timed_items().await;
        let lifecycle: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(90).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lifecycle, 2, "timer burns the expired ring");
        assert_eq!(
            hit(&owner, deps.actor, session, 5).await?,
            (985, 81),
            "expiry removes the shield even with mana available"
        );
        assert!(
            crate::gameplay_transport::equipment_move::observe_character_equipment(&owner, session)
                .await
                .ok_or("expired view")?
                .is_empty()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

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
