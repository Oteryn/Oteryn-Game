//! P08 command 378 through the real wire: select, refusals, replay across a reconnect and the
//! committed selection in the durable row and the session projection.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_protocol_oteryn::{
    self as protocol,
    weapon_proficiency::{SelectPerkReply, SelectPerkRequest, SelectPerkResult},
};
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const SWORD: &str = "oteryn:item.tibia.i3264";
const OTHER_WEAPON: &str = "oteryn:item.tibia.i3289";
const GOLD: &str = "oteryn:item.tibia.i3031";

#[test]
fn proficiency_select_perk_wire_refusals_replay_and_durable_selection() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency_select_perk".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency select scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "proficiency_select", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let reference = |key| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("item ref")
        };
        let (sword, other, gold) = (reference(SWORD)?, reference(OTHER_WEAPON)?, reference(GOLD)?);
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let shape = catalogue
            .resolve_weapon(SWORD)
            .ok_or("sword definition")?
            .shape;
        assert!(
            catalogue.resolve_weapon(OTHER_WEAPON).is_some(),
            "the second weapon has a promoted definition"
        );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let binding = owner.progression_binding(session).ok_or("progression")?;
        let definitions =
            catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;

        // Training progress that unlocks exactly the first perk level, checkpointed through
        // the production saver.
        let track_shape = shape.track_shape();
        let progress = (1..=u32::MAX / 2)
            .map(|step| step * 50)
            .find(|progress| track_shape.unlocked_perk_levels(*progress) == 1)
            .ok_or("first level threshold")?;
        {
            let mut states = owner.states.proficiencies.lock().map_err(|_| "states")?;
            let state = states.entry(session).or_default();
            let mut track = catalogue.empty_track(SWORD).ok_or("sword track")?;
            track.progress = progress;
            state.install_track(SWORD, track);
            state.pending_progress.insert(SWORD.to_owned(), progress);
            state.dirty_since = Some(0);
        }
        assert_eq!(
            owner.save_weapon_proficiency(actor, session, true).await,
            crate::gameplay_transport::actor_spell::TrainingSave::Saved
        );
        let in_zone = {
            let mut runtime = deps.runtime.lock().await;
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells,
                actor,
            ) == Some(true)
        };

        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(session)
        .await?
        .0;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 195],
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(195));

        let select = |definition: NonZeroU32, level, perk| {
            SelectPerkRequest {
                definition,
                level,
                perk,
            }
            .encode()
        };
        let levels = usize::from(track_shape.perk_levels());
        let mut chosen = vec![None; levels];
        chosen[0] = Some(0);
        let empty = vec![None; levels];
        let pool = &harness.pool;
        let receipts = || async {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_proficiency_receipts")
                .fetch_one(pool)
                .await
        };
        let before_receipts = receipts().await?;

        let accepted = protocol::CommandStatus::Accepted;
        let served = connection(&owner, admitted, async |client| {
            // Malformed and non-weapon payloads are closed refusals before any read.
            assert_eq!(
                send(client, 1, &[0, 0, 0, 1, 0], accepted).await?,
                reply(SelectPerkResult::Rejected, &[])
            );
            assert_eq!(
                send(client, 2, &select(gold, 0, Some(0)), accepted).await?,
                reply(SelectPerkResult::Rejected, &[])
            );
            assert_eq!(
                send(client, 3, &select(other, 0, Some(0)), accepted).await?,
                reply(SelectPerkResult::UnknownTrack, &[])
            );
            assert_eq!(
                send(client, 4, &select(sword, 1, Some(0)), accepted).await?,
                reply(SelectPerkResult::NotUnlocked, &empty)
            );
            assert_eq!(
                send(client, 5, &select(sword, 0, Some(9)), accepted).await?,
                reply(SelectPerkResult::InvalidPerk, &empty)
            );
            assert_eq!(receipts().await?, before_receipts);
            // An empty level is selected anywhere.
            assert_eq!(
                send(client, 6, &select(sword, 0, Some(0)), accepted).await?,
                reply(SelectPerkResult::Accepted, &chosen)
            );
            assert_eq!(receipts().await?, before_receipts + 1);
            // The foundation replays the retained result; nothing executes again.
            assert_eq!(
                send(
                    client,
                    6,
                    &select(sword, 0, Some(0)),
                    protocol::CommandStatus::DuplicateReplay
                )
                .await?,
                reply(SelectPerkResult::Accepted, &chosen)
            );
            // The same perk again is no change and no receipt.
            assert_eq!(
                send(client, 7, &select(sword, 0, Some(0)), accepted).await?,
                reply(SelectPerkResult::Accepted, &chosen)
            );
            assert_eq!(receipts().await?, before_receipts + 1);
            // Changing or clearing a selected perk needs a protection zone (TibiaWiki, Global).
            if !in_zone {
                assert_eq!(
                    send(client, 8, &select(sword, 0, None), accepted).await?,
                    reply(SelectPerkResult::NotInProtectionZone, &chosen)
                );
                assert_eq!(receipts().await?, before_receipts + 1);
            }
            Ok(())
        })
        .await?;
        assert!(served);
        let projected = owner
            .states
            .proficiencies
            .lock()
            .map_err(|_| "states")?
            .get(&session)
            .and_then(|state| state.track(SWORD).map(|track| track.selections.clone()));
        assert_eq!(projected.as_deref(), Some(&chosen[..]));

        // Without the foundation's retained result (a lost owner), the committed occurrence
        // answers command 6 again without a second receipt and refuses another choice.
        let command = crate::gameplay_transport::connection::UseCommand {
            game_session_id: session,
            command_id: 6,
            item_fence: admitted.item_fence,
        };
        let request = |perk| SelectPerkRequest {
            definition: sword,
            level: 0,
            perk,
        };
        assert_eq!(
            FreshAdmissionAuthority::weapon_proficiency_select(
                &owner,
                actor,
                command,
                request(Some(0))
            )
            .await,
            Some(reply(SelectPerkResult::Accepted, &chosen))
        );
        assert_eq!(
            FreshAdmissionAuthority::weapon_proficiency_select(&owner, actor, command, request(None))
                .await,
            Some(reply(SelectPerkResult::Conflict, &chosen))
        );
        assert_eq!(receipts().await?, before_receipts + 1);
        if in_zone {
            let command = crate::gameplay_transport::connection::UseCommand {
                command_id: 9,
                ..command
            };
            assert_eq!(
                FreshAdmissionAuthority::weapon_proficiency_select(
                    &owner,
                    actor,
                    command,
                    request(None)
                )
                .await,
                Some(reply(SelectPerkResult::Accepted, &empty))
            );
            chosen = empty.clone();
        }

        let (tracks, _) = harness
            .root
            .read_character_proficiency_with_definitions(
                &authority,
                current_character(&owner, session).await?,
                None,
                definitions,
            )
            .await
            .map_err(debug)?;
        let stored = tracks
            .iter()
            .find(|track| track.state.item_key() == SWORD)
            .ok_or("durable track")?;
        assert_eq!(stored.state.selections(), &chosen[..]);
        assert_eq!(stored.state.progress(), u64::from(progress));
        eprintln!("proficiency_select_perk: wire cases executed (in_zone={in_zone})");
        Ok(())
    })
}

fn reply(result: SelectPerkResult, selections: &[Option<u8>]) -> SelectPerkReply {
    SelectPerkReply {
        result,
        selections: selections.to_vec(),
    }
}

async fn current_character(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<crate::domain::CharacterId> {
    Ok(owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?
        .character_id)
}

/// Serve one admitted connection, drive it to its snapshot, run `exercise`, then close it.
async fn connection<F>(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    exercise: F,
) -> TestResult<bool>
where
    F: AsyncFnOnce(&mut tokio::io::DuplexStream) -> TestResult,
{
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    );
    let drive = async move {
        loop {
            let frame = item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                break;
            }
        }
        exercise(&mut client).await?;
        client.shutdown().await?;
        TestResult::Ok(())
    };
    let mut serve = std::pin::pin!(serve);
    let mut drive = std::pin::pin!(drive);
    let mut driven = None;
    std::future::poll_fn(|cx| {
        use std::task::Poll;
        if driven.is_none()
            && let Poll::Ready(value) = drive.as_mut().poll(cx)
        {
            if let Err(error) = value {
                return Poll::Ready(Err(error));
            }
            driven = Some(());
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if driven.is_some() {
                Ok(true)
            } else {
                Err(format!("connection ended before the exercise: {end:?}").into())
            });
        }
        Poll::Pending
    })
    .await
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
    status: protocol::CommandStatus,
) -> TestResult<SelectPerkReply> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: protocol::weapon_proficiency::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.status, status);
            return SelectPerkReply::decode(result.payload).ok_or_else(|| "reply".into());
        }
    }
}
