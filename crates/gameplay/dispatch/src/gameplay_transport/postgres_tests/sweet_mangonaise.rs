use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, SessionContinuity,
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn sweet_mangonaise_wire_use_copies_equipped_time_ring() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Sweet Mangonaise PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
        let h = Harness::create_with_admission_seed(admin, "sweet_mangonaise", true, false).await?;
        eprintln!("POSTGRES_STARTED SWEET-MANGONAISE");
        super::reset();
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
        let elixir = "oteryn:item.tibia.i11588";
        let ring = "oteryn:item.tibia.i3090";
        super::item_look::seed_main_backpack(&h, &[(elixir, 1), (ring, 1)]).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::raw_sql("
            SET LOCAL session_replication_role = replica;
            INSERT INTO game_character_equipment_state(character_id,revision)
            SELECT character_id,1 FROM game_item_container_slots;
            WITH worn AS (
                DELETE FROM game_item_container_entries e USING game_item_instances i
                WHERE e.item_instance_id=i.item_instance_id
                  AND i.definition_production_key='oteryn:item.tibia.i3090'
                RETURNING e.character_id,e.item_instance_id,e.world_id,e.placed_transaction_id
            )
            INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id)
            SELECT character_id,8,item_instance_id,world_id,placed_transaction_id FROM worn;
        ").execute(&mut *tx).await?;
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
                selected_capabilities:
                    crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[4, 6, 11, 12, 15, 18],
                    )
                    .ok_or("capabilities")?,
                ..SessionContinuity::FRESH
            },
            runtime_actor: Some(deps.actor),
            ..deps.admitted
        };
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let (_, disposition, _) = super::rust_lottery::exchange(
            &owner,
            admitted,
            1,
            &payload,
            index.definition_ref("Item", elixir, "definition-r1"),
            None,
        )
        .await?;
        assert_eq!(disposition, UseDisposition::Committed);

        let backpack = h
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack")?;
        assert_eq!(backpack.entries.len(), 10);
        assert!(
            backpack
                .entries
                .iter()
                .all(|entry| entry.item.definition.production_key == ring
                    && entry.item.quantity == 1)
        );
        let equipped: String = sqlx::query_scalar(
            "SELECT i.definition_production_key FROM game_character_equipment_slots s
             JOIN game_item_instances i USING(item_instance_id) WHERE s.slot=8",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(equipped, ring);
        Ok(())
    })
}
