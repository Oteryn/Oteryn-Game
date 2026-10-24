//! Real reset finalization -> admitted command bytes -> receipt -> result -> restart.
use super::*;
use crate::foundation::{
    ChannelId, MessageType, ScopeOwnershipGeneration, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_event as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

#[path = "world_event/reset_fixture.rs"]
mod event_fixture;

#[test]
fn world_event_postgres_real_dispatch_reset_restart_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for epochs in [1, 2] {
                scenario(epochs).map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "world event e2e panicked")??;
    Ok(())
}

fn scenario(epochs: u64) -> TestResult {
    run(async move |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "world_event_dispatch", true, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        assert_eq!(h.root.active_world_event_epoch(world).await?, None);
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let room = pin.qualified_room();
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        event_fixture::reset_epochs(&h, &authority, room, world, channel, epochs).await?;
        event_fixture::admit(&h, epochs + 1).await?;
        let mut deps = Dependencies::new()?;
        deps.room = room.clone();
        planned_world_reset::reset_fixture::set_runtime(
            &mut deps,
            world,
            channel,
            &pin,
            epochs + 1,
        )?;
        let actor = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            runtime.commit_fresh_session(reservation).map_err(debug)?
        };
        deps.actor = actor;
        deps.admitted.runtime_actor = Some(actor);
        deps.admitted.first_entry = FirstEntryOutcome::Positioned;
        let mut item_fence = fence(1)?.item_fence();
        item_fence.character_lease_generation = 2;
        item_fence.scope_ownership_generation =
            ScopeOwnershipGeneration::new(epochs + 1).map_err(debug)?;
        deps.admitted.item_fence = Some(item_fence);
        let mut admitted = deps.admitted;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        // Announcement queries use the shared owner's closed room gates.
        owner.reset_epoch = h
            .root
            .active_world_event_epoch(world)
            .await?
            .ok_or("active epoch")?;
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let offered = owner.offered_capabilities();
        assert!(
            offered
                .iter()
                .any(|c| c.id == wire::CAPABILITY_WORLD_EVENT_V1)
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, 53]).ok_or("capability 53")?;

        // Malformed query never reaches storage. A valid query commits its exact result.
        let results = exchange(&owner, admitted, &[(1, &[0]), (2, &[])]).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (CommandStatus::Rejected, Vec::new()));
        assert_eq!(results[1].0, CommandStatus::Accepted);
        let state = wire::decode_world_event_state(&results[1].1).map_err(debug)?;
        assert_eq!(state.reset_epoch, epochs);
        assert_eq!(state.active, epochs == 1);
        assert_eq!(state.announcement.is_empty(), epochs == 2);
        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT result FROM game_world_event_query_receipts WHERE command_id=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stored, results[1].1);
        assert_eq!(receipt_count(&h).await?, 1);

        // No negotiated capability => no event bytes or receipt, even for a valid query.
        let mut unsupported = admitted;
        unsupported.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6]).ok_or("base capability")?;
        assert_eq!(
            exchange(&owner, unsupported, &[(3, &[])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        assert_eq!(receipt_count(&h).await?, 1);

        // Fresh DB reads after reconstructing the runtime owner preserve exact bytes.
        drop(owner);
        let mut restarted = deps.owner(&h, &authority);
        restarted.active_generation = controller.active();
        restarted.reset_epoch = h
            .root
            .active_world_event_epoch(world)
            .await?
            .ok_or("restart epoch")?;
        let replay = exchange(&restarted, admitted, &[(2, &[]), (3, &[])]).await?;
        assert_eq!(
            replay,
            [
                (CommandStatus::Accepted, stored.clone()),
                (CommandStatus::Accepted, stored)
            ]
        );
        assert_eq!(receipt_count(&h).await?, 2);

        // Independently stale connection/lease/scope, wrong identity and missing fence.
        // Start at CommandId 1 so generic outstanding-Bank reconciliation does
        // not consume the stale fence before this mechanic's dispatch boundary.
        for which in 0..7 {
            let mut forged = admitted;
            let f = forged.item_fence.as_mut().ok_or("fence")?;
            match which {
                0 => {
                    f.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                1 => f.character_lease_generation += 1,
                2 => {
                    f.scope_ownership_generation =
                        ScopeOwnershipGeneration::new(epochs + 2).map_err(debug)?
                }
                3 => {
                    f.character_id =
                        crate::domain::CharacterId::from_bytes(id(99)).map_err(debug)?
                }
                4 => {
                    f.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(99)).map_err(debug)?,
                        channel,
                    )
                }
                5 => {
                    f.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                        world,
                        ChannelId::decode(&id(99)).map_err(debug)?,
                    )
                }
                _ => forged.item_fence = None,
            }
            assert_eq!(
                exchange(&restarted, forged, &[(1, &[])]).await?,
                [(CommandStatus::Rejected, Vec::new())],
                "negative fence {which} at epoch {epochs}"
            );
        }
        // Stale boot epoch and zero CommandId fail closed at their owning boundaries.
        restarted.reset_epoch = epochs + 1;
        assert_eq!(
            exchange(&restarted, admitted, &[(4, &[])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        assert!(
            h.root
                .query_world_event(&authority, &h.node, item_fence, 0, epochs)
                .await
                .is_err()
        );
        assert_eq!(receipt_count(&h).await?, 2);
        // Receipt lookup on a replay still requires the current session generation.
        let mut stale = item_fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .query_world_event(&authority, &h.node, stale, 2, epochs)
                .await
                .is_err()
        );
        // A failed query commit emits no terminal result; retrying its retained
        // CommandId after recovery commits exactly one receipt.
        restarted.reset_epoch = epochs;
        sqlx::query("CREATE FUNCTION reject_event_query() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected query failure' USING ERRCODE='23514'; END $$").execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER reject_event_query BEFORE INSERT ON game_world_event_query_receipts FOR EACH ROW EXECUTE FUNCTION reject_event_query()").execute(&h.pool).await?;
        assert!(
            exchange(&restarted, admitted, &[(4, &[])])
                .await?
                .is_empty()
        );
        assert_eq!(receipt_count(&h).await?, 2);
        sqlx::query("DROP TRIGGER reject_event_query ON game_world_event_query_receipts")
            .execute(&h.pool)
            .await?;
        let recovered = exchange(&restarted, admitted, &[(4, &[])]).await?;
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].0, CommandStatus::Accepted);
        assert_eq!(
            wire::decode_world_event_state(&recovered[0].1).map_err(debug)?,
            state
        );
        assert_eq!(receipt_count(&h).await?, 3);
        // Corrupt persisted state with just the event binding missing. Neither
        // channel boot nor real dispatch may synthesize a replacement decision. DAILY-1's
        // reset-history trigger needs the injected epoch to activate after the real ones.
        sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,99,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE reset_epoch=0").execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=99")
            .execute(&h.pool)
            .await?;
        assert_eq!(h.root.active_world_reset_epoch(world).await?, Some(99));
        assert!(h.root.active_world_event_epoch(world).await.is_err());
        restarted.reset_epoch = 99;
        assert!(
            exchange(&restarted, admitted, &[(5, &[])])
                .await?
                .is_empty()
        );
        assert_eq!(receipt_count(&h).await?, 3);
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$1::text::numeric")
            .bind(epochs.to_string())
            .execute(&h.pool)
            .await?;
        deps.loss(&h, 60).await?;
        assert!(matches!(h.root.query_world_event(&authority, &h.node, item_fence, 4, epochs).await,
            Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
        assert_eq!(receipt_count(&h).await?, 3);
        eprintln!("WORLD-EVENT-PG: real reset epoch={epochs}, wire/receipt/restart/fences passed");
        drop(restarted);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_world_event_query_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    commands: &[(u64, &[u8])],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    // Recreate the connection with its retained next/outstanding CommandId.
    if let Some((command_id, _)) = commands.first() {
        admitted.continuity.next_command_id = *command_id;
    }
    let mut input = Vec::new();
    for (command_id, payload) in commands {
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: *command_id,
                command_type: wire::COMMAND_TYPE_WORLD_EVENT_QUERY,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend_from_slice(&u32::try_from(frame.len())?.to_be_bytes());
        input.extend_from_slice(&frame);
    }
    let (_, output) = weather_wire::fixture_access::serve_input(owner, admitted, &input).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
        cursor += 4 + length;
    }
    Ok(results)
}
