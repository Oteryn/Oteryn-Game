//! Plain USE through the negotiated connection, backed by the real item owners.
use super::fixture::{Harness, debug, id, join_two};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::item_look_fixtures::{read_frame, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelContentPin, ExactActorRef, GameSessionId, MessageType,
    WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, ControlLossResult, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, ResumeAttempt, UseCommand,
    UseOutcome, serve_admitted,
};
use crate::gameplay_transport::item_view::{InventoryItems, ItemKey, SessionItemView};
use crate::gameplay_transport::world_map::{MapUpdate, MapViewError, SessionMapView};
use crate::gameplay_transport::{
    COMMAND_HOLDER_WINDOW, ComposedFreshAdmission, item_fence_of,
    world_spatial::WorldSpatialObservation,
};
use oteryn_gameplay_items::drome_cube::DROME_CUBE_KEY;
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
    encode_client_command,
};
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt as _, DuplexStream};

const GOLD: &str = "oteryn:item.tibia.i3031";
const PLATINUM: &str = "oteryn:item.tibia.i3035";

#[derive(Clone, Copy)]
enum Case {
    Gold,
    CubeShort,
    CubeExhausted,
    AidProbeExhausted,
    CubeCommitted,
    CoinCommitted,
    CoinNotApplied,
}

#[test]
fn s7c_plain_gold_use_converts_and_replays_with_rune_capability() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Gold)
}

#[test]
fn s7c_cube_use_survives_a_short_holder_checkout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CubeShort)
}

#[test]
fn s7c_cube_use_closes_and_logs_when_holder_stays_busy() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CubeExhausted)
}

#[test]
fn s7c_kooldown_aid_probe_exhaustion_keeps_cube_use_pending() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::AidProbeExhausted)
}

#[test]
fn s7c_resume_replays_committed_cube_after_lost_ack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CubeCommitted)
}

#[test]
fn s7c_resume_replays_committed_coin_with_retired_handle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CoinCommitted)
}

#[test]
fn s7c_resume_rejects_unapplied_coin_exchange_without_converting() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CoinNotApplied)
}

fn run(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            let mut result = Ok(());
            let lines = oteryn_error_codes::capture(|| {
                result = runtime.block_on(scenario(admin, case));
            });
            result.map_err(|error| error.to_string())?;
            for (stage, expected) in [
                (
                    "stage=drome_cube_receipt",
                    matches!(case, Case::CubeExhausted),
                ),
                (
                    "stage=kooldown_aid_receipt",
                    matches!(case, Case::AidProbeExhausted),
                ),
            ] {
                let exhausted = lines.iter().any(|line| {
                    line.contains("event=session_durability_failed")
                        && line.contains(stage)
                        && line.contains("kind=holder_busy")
                });
                assert_eq!(exhausted, expected, "{stage}");
            }
            Ok::<(), String>(())
        })?
        .join()
        .map_err(|_| "S7C PostgreSQL thread panicked")??;
    Ok(())
}

async fn scenario(admin: String, case: Case) -> TestResult {
    let (label, entries): (_, &[(&str, i64)]) = match case {
        Case::Gold => ("s7c_gold", &[(GOLD, 100), (GOLD, 99)]),
        Case::CubeShort => ("s7c_cube_short", &[(DROME_CUBE_KEY, 1)]),
        Case::CubeExhausted => ("s7c_cube_exhausted", &[(DROME_CUBE_KEY, 1)]),
        Case::AidProbeExhausted => ("s7c_aid_probe_exhausted", &[(DROME_CUBE_KEY, 1)]),
        Case::CubeCommitted => ("s7c_cube_committed", &[(DROME_CUBE_KEY, 1)]),
        Case::CoinCommitted => ("s7c_coin_committed", &[(GOLD, 100)]),
        Case::CoinNotApplied => ("s7c_coin_not_applied", &[(GOLD, 100)]),
    };
    let h = Harness::create_with_admission_seed(admin, label, true, false).await?;
    let result = exercise(&h, case, entries).await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup?;
    eprintln!("POSTGRES_CASE_COMPLETED {label}");
    Ok(())
}

async fn exercise(h: &Harness, case: Case, entries: &[(&str, i64)]) -> TestResult {
    let recovery = if matches!(case, Case::Gold | Case::CubeShort) {
        prepare_current_session(h).await?;
        None
    } else {
        let recovery = RecoverySource::start(h).await?;
        prepare_owned_session(h).await?;
        Some(recovery)
    };
    if matches!(case, Case::CoinNotApplied) {
        // Allow the accepted request to persist, then roll back its finalization.
        sqlx::raw_sql(
            "CREATE FUNCTION s7c_coin_no_commit() RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN RAISE EXCEPTION 's7c coin finalization refused' USING ERRCODE='23514'; END $$;
             CREATE TRIGGER s7c_coin_no_commit BEFORE UPDATE ON game_coin_exchange_receipts
                FOR EACH ROW EXECUTE FUNCTION s7c_coin_no_commit()",
        )
        .execute(&h.pool)
        .await?;
    }
    seed_main_backpack(h, entries).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let index = native.item_index().ok_or("item index")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let room = pin.qualified_room().clone();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 0, 7),
    );
    let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    let definition = |key: &str| {
        index
            .revision(key)
            .and_then(|revision| index.definition_ref("Item", key, revision))
            .ok_or("item definition")
    };
    let bag = "oteryn:item.tibia.i2854";
    let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
        (bag, definition(bag)?.get() - 1),
        (GOLD, definition(GOLD)?.get() - 1),
    );
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let map = crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        },
        world,
        deps.admitted.channel_id,
        |key| {
            let policy = native.item_policy(key, "definition-r1")?.record();
            let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical
            else {
                return None;
            };
            Some(crate::map::facts::ItemDefinition {
                reference: definition(key).ok()?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                door: false,
            })
        },
    )
    .map_err(debug)?;
    let cells = map
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    if let Some(recovery) = &recovery {
        owner.evidence = &recovery.evidence;
    }
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
    let fence = item_fence_of(current).ok_or("current item fence")?;
    admitted.item_fence = Some(fence);
    let supported = [4, 6, 11, 15, 18, 22, 171];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &supported)
            .ok_or("selected USE capabilities")?;
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        &supported
    );
    owner.states.command_ingress.admit(admitted.game_session_id);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let wire_owner = WireOwner {
        owner: &owner,
        case,
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &wire_owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let client_task = async {
        let inventory = read_inventory(&mut client).await?;
        match case {
            Case::Gold => {
                assert_gold_wire(&mut client, &inventory, definition(GOLD)?).await?;
                Ok::<_, Box<dyn std::error::Error>>(None)
            }
            Case::CubeShort => {
                send_use(
                    &mut client,
                    1,
                    1,
                    UseTarget::ItemByDefinition(definition(DROME_CUBE_KEY)?),
                )
                .await?;
                assert_eq!(
                    read_use(&mut client, 1).await?,
                    (CommandStatus::Accepted, UseDisposition::Committed)
                );
                client.shutdown().await?;
                Ok(None)
            }
            _ => {
                let target = if matches!(case, Case::CoinCommitted | Case::CoinNotApplied) {
                    let gold = definition(GOLD)?;
                    let stack = inventory
                        .iter()
                        .find(|item| item.item_definition_ref == gold && item.count.get() == 100)
                        .ok_or("full stack in initial view")?;
                    UseTarget::Item(stack.handle)
                } else {
                    UseTarget::ItemByDefinition(definition(DROME_CUBE_KEY)?)
                };
                send_use(&mut client, 1, 1, target.clone()).await?;
                assert_no_use_result(&mut client).await?;
                Ok(Some(target))
            }
        }
    };
    let (end, client_result) =
        tokio::time::timeout(Duration::from_secs(20), join_two(serving, client_task)).await?;
    let original_target = client_result?;
    let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
        return Err(format!("unexpected connection end: {end:?}").into());
    };
    match case {
        Case::Gold => {
            assert_eq!(retained.continuity.next_command_id, 3);
            assert_eq!(
                count(h, "SELECT count(*) FROM game_coin_exchange_receipts").await?,
                2
            );
            let bag = h
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await?
                .ok_or("durable backpack")?;
            let quantity = |key| {
                bag.entries
                    .iter()
                    .filter(|entry| entry.item.definition.production_key == key)
                    .map(|entry| entry.item.quantity)
                    .sum::<u32>()
            };
            assert_eq!(quantity(GOLD), 99);
            assert_eq!(quantity(PLATINUM), 1);
        }
        Case::CubeShort => {
            assert_eq!(retained.continuity.next_command_id, 2);
            assert_eq!(
                count(h, "SELECT count(*) FROM game_drome_cube_uses").await?,
                1
            );
        }
        Case::CubeExhausted | Case::AidProbeExhausted => {
            assert_eq!(
                count(h, "SELECT count(*) FROM game_drome_cube_uses").await?,
                0
            );
        }
        Case::CubeCommitted => assert_eq!(
            count(h, "SELECT count(*) FROM game_drome_cube_uses").await?,
            1
        ),
        Case::CoinCommitted => assert_eq!(
            count(
                h,
                "SELECT count(*) FROM game_coin_exchange_receipts WHERE result=1"
            )
            .await?,
            1
        ),
        Case::CoinNotApplied => assert_eq!(
            count(
                h,
                "SELECT count(*) FROM game_coin_exchange_receipts WHERE result=0"
            )
            .await?,
            1
        ),
    }
    if let Some(target) = original_target {
        assert!(!owner.states.command_ingress.resumable(&retained));
        assert_eq!(
            owner.lose_control(retained, Duration::ZERO, None).await,
            ControlLossResult::Recorded
        );
        let token = RecoverySource::token(2)?;
        let resumed = owner
            .resume(ResumeAttempt {
                game_session_id: retained.game_session_id,
                recovery_material: token.as_bytes(),
                transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
                last_applied_server_sequence: retained.continuity.server_sequence,
                supported_capabilities: &supported,
            })
            .await
            .map_err(|error| format!("pending USE resume: {error:?}"))?;
        assert!(owner.states.command_ingress.resumable(&resumed));
        assert_eq!(resumed.continuity.next_command_id, 2);
        let committed = matches!(case, Case::CubeCommitted | Case::CoinCommitted);
        let expected = if committed {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            let end = serve_admitted(&mut server, resumed, &wire_owner, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let replay = async {
            let inventory = read_inventory(&mut client).await?;
            if let UseTarget::Item(old) = target {
                assert!(inventory.iter().all(|item| item.handle > old));
                let gold = definition(GOLD)?;
                assert_eq!(
                    inventory
                        .iter()
                        .filter(|item| item.item_definition_ref == gold)
                        .map(|item| item.count.get())
                        .sum::<u32>(),
                    if committed { 0 } else { 100 },
                );
            } else {
                let cube = definition(DROME_CUBE_KEY)?;
                assert_eq!(
                    inventory
                        .iter()
                        .filter(|item| item.item_definition_ref == cube)
                        .map(|item| item.count.get())
                        .sum::<u32>(),
                    if committed { 0 } else { 1 },
                );
            }
            send_use(&mut client, 2, 1, target).await?;
            assert_eq!(
                read_use(&mut client, 1).await?,
                (CommandStatus::DuplicateReplay, expected)
            );
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (end, replay) =
            tokio::time::timeout(Duration::from_secs(20), join_two(serving, replay)).await?;
        replay?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    }
    Ok(())
}

async fn count(h: &Harness, query: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(query).fetch_one(&h.pool).await?)
}

struct WireOwner<'a, 'b, 'f, 's> {
    owner: &'a ComposedFreshAdmission<'b, 'f, 's>,
    case: Case,
}

impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        self.owner.session_commands()
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.owner.observe(actor).await
    }

    async fn observe_character_inventory(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<InventoryItems> {
        self.owner.observe_character_inventory(actor, session).await
    }

    async fn observe_world_map(
        &self,
        actor: ExactActorRef,
        view: &mut SessionMapView,
        items: &mut SessionItemView,
    ) -> Option<Result<Option<MapUpdate>, MapViewError>> {
        self.owner.observe_world_map(actor, view, items).await
    }

    async fn use_item_completion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<UseOutcome> {
        let result = self
            .owner
            .use_item_completion(actor, command, item)
            .await;
        if matches!(self.case, Case::Gold) && command.command_id == 1 {
            // The connection cache would mask a second owner call. Model an ACK lost
            // before an inventory refresh, so the original handle still names the
            // retired instance when the same command reaches the coin owner again.
            assert_eq!(
                result.as_ref().map(|outcome| outcome.disposition),
                Some(UseDisposition::Committed)
            );
            let replay = self
                .owner
                .use_coin(actor, command, item, true)
                .await
                .flatten();
            assert_eq!(
                replay.map(|outcome| outcome.disposition),
                Some(UseDisposition::Committed)
            );
        }
        if matches!(self.case, Case::CoinCommitted) {
            assert_eq!(
                result.map(|outcome| outcome.disposition),
                Some(UseDisposition::Committed)
            );
            return None;
        }
        result
    }

    async fn use_drome_cube(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let started = Instant::now();
        let using = async {
            let result = if matches!(self.case, Case::CubeExhausted) {
                // Isolate the second probe. The composed entry reaches Kooldown Aid first.
                self.owner
                    .use_drome_cube_impl(actor, command, intent, source)
                    .await
            } else {
                self.owner
                    .use_drome_cube(actor, command, intent, source)
                    .await
            };
            if matches!(self.case, Case::CubeExhausted | Case::AidProbeExhausted) {
                assert!(started.elapsed() >= COMMAND_HOLDER_WINDOW / 2);
            }
            result
        };
        let holding = match self.case {
            Case::CubeCommitted => {
                crate::gameplay_transport::item_use::drome_cube::test_faults::lose_next_ack();
                return using.await;
            }
            Case::Gold | Case::CoinCommitted | Case::CoinNotApplied => return using.await,
            Case::CubeShort => Duration::from_millis(100),
            Case::CubeExhausted | Case::AidProbeExhausted => {
                COMMAND_HOLDER_WINDOW + Duration::from_millis(200)
            }
        };
        let mut holder = self.owner.root.try_acquire_ready().expect("fixture holder");
        assert!(!self.owner.root.is_ready());
        let releasing = async {
            tokio::time::sleep(holding).await;
            holder
                .return_to_pool_observed_until(Instant::now() + crate::durability::DB_PASS_DEADLINE)
                .await;
        };
        let (outcome, ()) = join_two(using, releasing).await;
        outcome
    }
}

async fn read_inventory(
    client: &mut DuplexStream,
) -> TestResult<Vec<oteryn_protocol_oteryn::item_view::ItemEntry>> {
    let mut entries = Vec::new();
    loop {
        let bytes = read_frame(client).await?;
        let frame = decode_wire_envelope(&bytes)?;
        if frame.message_type() == MessageType::SnapshotCommit {
            return Ok(entries);
        }
        if frame.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(frame.payload())?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    entries = decode_character_inventory(domain.payload)
                        .map_err(debug)?
                        .entries;
                }
            }
        }
    }
}

async fn assert_gold_wire(
    client: &mut DuplexStream,
    inventory: &[oteryn_protocol_oteryn::item_view::ItemEntry],
    gold: NonZeroU32,
) -> TestResult {
    let target = |count| {
        inventory
            .iter()
            .find(|entry| entry.item_definition_ref == gold && entry.count.get() == count)
            .map(|entry| UseTarget::Item(entry.handle))
    };
    let full = target(100).ok_or("full gold stack")?;
    let short = target(99).ok_or("short gold stack")?;
    send_use(client, 1, 1, full.clone()).await?;
    assert_eq!(
        read_use(client, 1).await?,
        (CommandStatus::Accepted, UseDisposition::Committed)
    );
    send_use(client, 1, 1, full).await?;
    assert_eq!(
        read_use(client, 1).await?,
        (CommandStatus::DuplicateReplay, UseDisposition::Committed)
    );
    send_use(client, 1, 2, short).await?;
    assert_eq!(
        read_use(client, 2).await?,
        (CommandStatus::Accepted, UseDisposition::NotPossible)
    );
    client.shutdown().await?;
    Ok(())
}

async fn assert_no_use_result(client: &mut DuplexStream) -> TestResult {
    loop {
        match read_frame(client).await {
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::UnexpectedEof) =>
            {
                break;
            }
            Err(error) => return Err(error),
            Ok(bytes) => assert_ne!(
                decode_wire_envelope(&bytes)?.message_type(),
                MessageType::CommandResult
            ),
        }
    }
    client.shutdown().await?;
    Ok(())
}

async fn send_use(
    client: &mut DuplexStream,
    generation: u64,
    command_id: u64,
    target: UseTarget,
) -> TestResult {
    let payload = encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_USE_INTENT,
            payload: &payload,
        },
    )?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn read_use(
    client: &mut DuplexStream,
    command_id: u64,
) -> TestResult<(CommandStatus, UseDisposition)> {
    loop {
        let bytes = read_frame(client).await?;
        let frame = decode_wire_envelope(&bytes)?;
        if frame.message_type() == MessageType::CommandResult {
            let result = decode_command_result(frame.payload())?;
            assert_eq!(result.command_id, command_id);
            return Ok((
                result.status,
                decode_use_result(result.payload).map_err(debug)?,
            ));
        }
    }
}
