//! Common ingress through the production connection and an existing atomic family writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::foundation::{CharacterLease, MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence, item_fence_of};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, appearance, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn session_ingress_postgres_real_dispatch_replay_and_recovery() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("SESSION-INGRESS-1: no PostgreSQL route; run in the PostgreSQL harness lane");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "session ingress e2e panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let mut h = Harness::create_appearance(admin, "session_ingress_wire").await?;
        eprintln!("SESSION-INGRESS-1: PostgreSQL harness created; exercising production dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[6, appearance::CAPABILITY_APPEARANCE_V1])
                .ok_or("appearance offered by active content")?;
        let entity = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: deps.actor.placement_identity(),
            generation: deps.actor.actor_local_generation(),
        };
        let initial = owner
            .appearance_observation(deps.actor, admitted.game_session_id, vec![entity])
            .await
            .ok_or("initial appearance")?;
        let change = appearance::Change {
            context: initial.catalogue.context,
            expected_root_revision: 1,
            prior: None,
            selection: appearance::Selection {
                outfit_key: initial
                    .catalogue
                    .outfits
                    .first()
                    .ok_or("outfit")?
                    .key
                    .clone(),
                colours: [1, 2, 3, 4],
                addons: 0,
                mount_key: None,
            },
            content_revision: "content-1".into(),
        };
        let payload = change.encode().map_err(debug)?;
        assert_eq!(initial.catalogue.root_revision, 1);
        let current_fence =
            character_gameplay_fence::current(&owner, admitted.game_session_id, false)
                .await
                .ok_or("initial gameplay fence")?;
        assert_eq!(admitted.item_fence, Some(current_fence.item_fence()));
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .command_selected(appearance::COMMAND_TYPE_APPEARANCE_CHANGE)
        );
        // Replacement bytes deliberately name an invalid input. CommandId 1's original
        // result is replayed; none of these bytes reach the appearance decoder/writer.
        let (ended, results) = exchange(
            &owner,
            admitted,
            &[
                (
                    1,
                    appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
                    payload.as_slice(),
                ),
                (1, appearance::COMMAND_TYPE_APPEARANCE_CHANGE, &[0xff]),
            ],
        )
        .await?;
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].0,
            CommandStatus::Accepted,
            "wire results: {results:?}"
        );
        assert_eq!(results[1].0, CommandStatus::DuplicateReplay);
        assert_eq!(results[0].1, results[1].1);
        assert_eq!(
            appearance::ChangeResult::decode(&results[0].1).map_err(debug)?,
            appearance::ChangeResult::Committed(2)
        );
        assert_eq!(ended.continuity.next_command_id, 2);
        assert_eq!(receipt_count(&h).await?, 1);
        assert!(owner.states.command_ingress.resumable(&ended));
        // A new connection recovers the Foundation stream and retained result, then
        // accepts the next ID. The second command's semantic refusal is terminal too.
        let (ended, recovered) = exchange(
            &owner,
            ended,
            &[
                (1, appearance::COMMAND_TYPE_APPEARANCE_CHANGE, &[0xff]),
                (2, appearance::COMMAND_TYPE_APPEARANCE_CHANGE, &[0xff]),
                (
                    2,
                    appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
                    payload.as_slice(),
                ),
            ],
        )
        .await?;
        assert_eq!(recovered.len(), 3);
        assert_eq!(
            recovered[0],
            (CommandStatus::DuplicateReplay, results[0].1.clone())
        );
        assert_eq!(recovered[1].0, CommandStatus::Rejected);
        assert_eq!(recovered[2].0, CommandStatus::DuplicateReplay);
        assert_eq!(recovered[1].1, recovered[2].1);
        assert_eq!(ended.continuity.next_command_id, 3);
        assert_eq!(receipt_count(&h).await?, 1);
        let fence = character_gameplay_fence::current(&owner, admitted.game_session_id, false)
            .await
            .ok_or("current appearance fence")?;
        let selected = owner
            .root
            .read_current_character_appearance(owner.character, owner.holder, fence)
            .await
            .map_err(debug)?
            .ok_or("durable selection")?;
        assert_eq!(selected.selection.colours, [1, 2, 3, 4]);
        assert_eq!(selected.committed_character_revision.get(), 2);
        drop(owner);
        drop(authority);
        drop(seal);
        h.restart_appearance_root().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        let selected = owner
            .root
            .read_current_character_appearance(owner.character, owner.holder, fence)
            .await
            .map_err(debug)?
            .ok_or("selection after restart")?;
        assert_eq!(selected.committed_character_revision.get(), 2);
        assert_eq!(selected.selection.colours, [1, 2, 3, 4]);
        // A rebuilt owner has no proven old command stream; it must not fabricate one
        // from continuity, even though the durable family result survived its restart.
        assert!(!owner.states.command_ingress.resumable(&ended));
        assert_eq!(receipt_count(&h).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    commands: &[(u64, u32, &[u8])],
) -> TestResult<(AdmittedSession, Vec<(CommandStatus, Vec<u8>)>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for &(command_id, command_type, payload) in commands {
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id,
                command_type,
                payload,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
        return Err(format!("unexpected connection end: {end:?}").into());
    };
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut results = Vec::new();
    let mut cursor = bytes.as_slice();
    let mut sequence = admitted.continuity.server_sequence;
    while !cursor.is_empty() {
        let length = usize::try_from(u32::from_be_bytes(cursor[..4].try_into()?))?;
        let envelope = decode_wire_envelope(&cursor[4..4 + length]).map_err(debug)?;
        assert_eq!(
            envelope.connection_generation(),
            admitted.continuity.connection_generation
        );
        if envelope.server_sequence() != 0 {
            sequence += 1;
            assert_eq!(envelope.server_sequence(), sequence);
        }
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
        cursor = &cursor[4 + length..];
    }
    assert_eq!(sequence, ended.continuity.server_sequence);
    Ok((ended, results))
}
