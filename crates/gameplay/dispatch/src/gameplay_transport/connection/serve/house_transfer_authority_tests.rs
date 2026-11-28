use super::*;
use std::cell::Cell;
use std::time::Duration;

struct Authority {
    staff: bool,
    checks: Cell<usize>,
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn staff_refresh(&self, _: AdmittedSession) -> bool {
        if self.staff {
            self.checks.set(self.checks.get() + 1);
            return false;
        }
        true
    }

    async fn account_closure_check(&self, _: AdmittedSession) -> bool {
        if !self.staff {
            self.checks.set(self.checks.get() + 1);
            return true;
        }
        false
    }
}

#[test]
fn house_idle_rechecks_staff_and_account_closure() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    for staff in [false, true] {
                        let authority = Authority { staff, checks: Cell::new(0) };
                        let id = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
                        let world_id = WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
                        let channel_id = ChannelId::decode(&id).map_err(|e| format!("{e:?}"))?;
                        let admitted = AdmittedSession {
                            game_session_id: GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?,
                            world_id, channel_id,
                            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                            first_entry: FirstEntryOutcome::Positioned,
                            controller: None, item_fence: None, continuity: SessionContinuity::FRESH,
                        };
                        let (mut server, _client) = tokio::io::duplex(1 << 16);
                        let mut messages = None;
                        let mut integrity = integrity::Integrity::default();
                        let result = tokio::time::timeout(Duration::from_secs(6), serve(
                            &mut server, admitted, &authority, &authority, &mut messages,
                            IDLE_LIVENESS, &mut integrity, false,
                        )).await;
                        assert!(result.is_ok(), "House failed to recheck authority: staff={staff}");
                        assert!(matches!(result, Ok(Err(end)) if matches!(*end, ConnectionEnd::AdmittedThenDisconnected(_))));
                        assert_eq!(authority.checks.get(), 1);
                    }
                    Ok(())
                })
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "House authority test panicked".to_owned())?
}

struct CommandAuthority {
    reserved: bool,
    move_seen: Cell<Option<crate::gameplay_transport::item_move::LoopMove>>,
}

impl FreshAdmissionAuthority for CommandAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [1; 32],
            actor_position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        })
    }

    async fn mana_shield_remaining(&self, _: AdmittedSession) -> Option<u32> {
        Some(37)
    }

    async fn server_info_query(&self, _: AdmittedSession, payload: &[u8]) -> Option<Vec<u8>> {
        use oteryn_protocol_oteryn::server_info::{Query, Reply};
        assert_eq!(Query::decode(payload), Some(Query::Quests));
        Reply::Quests(vec![(1, "A House quest".into())]).encode()
    }

    async fn mount(&self, _: ExactActorRef, _: UseCommand, riding: bool) -> Option<bool> {
        assert!(riding);
        Some(true)
    }

    async fn item_inscription_read(
        &self,
        _: AdmittedSession,
        item: Option<crate::gameplay_transport::item_view::ItemKey>,
        attributed: bool,
    ) -> Option<Vec<u8>> {
        assert_eq!(
            item,
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(
                [2; 16]
            ))
        );
        assert!(!attributed);
        oteryn_protocol_oteryn::item_inscription::Reply {
            text: "Resident letter".into(),
            max_length: 1999,
            writable: true,
            writer_name: None,
            written_at_unix_ms: None,
        }
        .encode()
    }

    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        use crate::gameplay_transport::item_view::{InventoryItems, ItemKey, ViewItem};
        let definition = std::num::NonZeroU32::new(1)?;
        let item = |id| ViewItem {
            key: ItemKey::Instance([id; 16]),
            appearance_id: 0,
            item_definition_ref: definition,
            count: definition,
            sub_type: 0,
        };
        Some(InventoryItems {
            main_backpack: Some(item(1)),
            entries: vec![item(2), item(3)],
            equipment: Vec::new(),
        })
    }

    async fn player_trade_item_available(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        item: Option<crate::gameplay_transport::item_view::ItemKey>,
    ) -> Option<bool> {
        assert_eq!(
            item,
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(
                [2; 16]
            ))
        );
        Some(!self.reserved)
    }

    async fn move_loop_item(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        request: crate::gameplay_transport::item_move::LoopMove,
    ) -> crate::gameplay_transport::item_move::LoopMoveResult {
        self.move_seen.set(Some(request));
        crate::gameplay_transport::item_move::LoopMoveResult::Committed
    }
}

fn command_fixture(ids: &[u32]) -> Result<(AdmittedSession, ExactActorRef), String> {
    let id = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
    let world_id = WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
    let channel_id = ChannelId::decode(&id).map_err(|e| format!("{e:?}"))?;
    let actor = ExactActorRef::transport_fixture(world_id, channel_id);
    let offers = ids
        .iter()
        .map(|&id| crate::gameplay_transport::capabilities::OfferedCapability { id, requires: &[] })
        .collect::<Vec<_>>();
    Ok((
        AdmittedSession {
            game_session_id: GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?,
            world_id,
            channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            item_fence: None,
            continuity: SessionContinuity {
                selected_capabilities: SelectedCapabilities::select(&offers, ids)
                    .ok_or("capabilities")?,
                ..SessionContinuity::FRESH
            },
        },
        actor,
    ))
}

fn run_command_test<F>(make: impl FnOnce() -> F + Send + 'static) -> Result<(), String>
where
    F: std::future::Future<Output = Result<(), String>>,
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(make())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "House command test panicked".to_owned())?
}

#[test]
fn house_query_mount_and_inscription_routes_return_owner_results() -> Result<(), String> {
    run_command_test(|| async {
        use oteryn_protocol_oteryn::{actor_motion, item_inscription, server_info};
        let (mut admitted, actor) = command_fixture(&[4, 500, 502, 511])?;
        let owner = CommandAuthority {
            reserved: false,
            move_seen: Cell::new(None),
        };
        let (mut state, _) = house_hireling::State::prepare(&owner, actor, &mut admitted)
            .await
            .map_err(|_| "prepare")?;
        let query = server_info::Query::Quests.encode().ok_or("quest query")?;
        let mount = actor_motion::encode_mount(true);
        let read = item_inscription::Request { item_handle: 2 }
            .encode()
            .ok_or("read request")?;
        let expected_quests = server_info::Reply::Quests(vec![(1, "A House quest".into())])
            .encode()
            .ok_or("quest reply")?;
        let expected_text = item_inscription::Reply {
            text: "Resident letter".into(),
            max_length: 1999,
            writable: true,
            writer_name: None,
            written_at_unix_ms: None,
        }
        .encode()
        .ok_or("letter reply")?;
        for (command_type, payload, expected) in [
            (
                server_info::COMMAND_TYPE_SERVER_INFO_QUERY,
                query.as_slice(),
                expected_quests,
            ),
            (
                actor_motion::COMMAND_TYPE_MOUNT,
                mount.as_slice(),
                Vec::new(),
            ),
            (
                item_inscription::COMMAND_TYPE_ITEM_INSCRIPTION_READ,
                read.as_slice(),
                expected_text,
            ),
        ] {
            let command = oteryn_protocol_oteryn::ClientCommandView {
                command_type,
                command_id: 1,
                payload,
            };
            assert!(matches!(state.route(&owner, &mut admitted, command).await,
                Some(Dispatch::Reply(CommandStatus::Accepted, reply)) if reply == expected));
        }
        Ok(())
    })
}

#[test]
fn house_live_state_reports_own_shield_and_no_follow() -> Result<(), String> {
    run_command_test(|| async {
        let (mut admitted, actor) = command_fixture(&[605])?;
        let owner = CommandAuthority {
            reserved: false,
            move_seen: Cell::new(None),
        };
        let (mut state, _) = house_hireling::State::prepare(&owner, actor, &mut admitted)
            .await
            .map_err(|_| "prepare")?;
        for (command_type, expected) in [(6371, vec![]), (6372, 37_u32.to_le_bytes().to_vec())] {
            let command = oteryn_protocol_oteryn::ClientCommandView {
                command_type,
                command_id: 1,
                payload: &[],
            };
            assert!(matches!(state.route(&owner, &mut admitted, command).await,
                Some(Dispatch::Reply(CommandStatus::Accepted, reply)) if reply == expected));
            let invalid = oteryn_protocol_oteryn::ClientCommandView {
                payload: &[0],
                ..command
            };
            assert!(matches!(state.route(&owner, &mut admitted, invalid).await,
                Some(Dispatch::Reply(CommandStatus::Rejected, reply)) if reply.is_empty()));
        }
        Ok(())
    })
}

#[test]
fn house_slotted_move_reaches_owner_with_or_without_hirelings() -> Result<(), String> {
    run_command_test(|| async {
        use oteryn_protocol_oteryn::item_view::{self, ItemMoveDestination, ItemMoveIntent};
        for ids in [&[4, 14, 510][..], &[4, 14, 165, 510]] {
            let (mut admitted, actor) = command_fixture(ids)?;
            let owner = CommandAuthority {
                reserved: false,
                move_seen: Cell::new(None),
            };
            let (mut state, _) = house_hireling::State::prepare(&owner, actor, &mut admitted)
                .await
                .map_err(|_| "prepare")?;
            let payload = item_view::encode_item_move_intent_for(
                &ItemMoveIntent {
                    source: std::num::NonZeroU64::new(2).ok_or("source")?,
                    destination: ItemMoveDestination::Container(
                        std::num::NonZeroU64::new(3).ok_or("destination")?,
                        Some(2),
                    ),
                },
                ItemMoveSelection {
                    container_tree: true,
                    manual_sort: true,
                    ..ItemMoveSelection::default()
                },
            )
            .map_err(|e| format!("{e:?}"))?;
            let command = oteryn_protocol_oteryn::ClientCommandView {
                command_type: 9,
                command_id: 1,
                payload: &payload,
            };
            let dispatch = state
                .route(&owner, &mut admitted, command)
                .await
                .ok_or("move dispatch")?;
            assert!(matches!(
                dispatch,
                Dispatch::ItemMove(ItemMoveOutcome::Moved, _, _)
            ));
            assert_eq!(owner.move_seen.get().ok_or("move owner")?.slot, Some(2));
            assert_eq!(
                house_hireling::result(admitted.continuity.selected_capabilities, &dispatch)
                    .map_err(|_| "move reply")?
                    .0,
                CommandStatus::Accepted
            );
        }
        Ok(())
    })
}

#[test]
fn counted_move_checks_trade_reservation_before_reaching_writer() -> Result<(), String> {
    run_command_test(|| async {
        let (mut admitted, actor) = command_fixture(&[4, 60, 61, 501])?;
        let owner = CommandAuthority {
            reserved: true,
            move_seen: Cell::new(None),
        };
        let mut view = SessionItemView::resume(admitted.continuity.item_view);
        let inventory = owner
            .observe_character_inventory(actor, admitted.game_session_id)
            .await
            .ok_or("inventory")?;
        view.snapshot(inventory, None)
            .map_err(|e| format!("{e:?}"))?;
        let selected = admitted.continuity.selected_capabilities;
        let dispatch = routing::item_move::route(
            &owner,
            actor,
            &mut admitted,
            oteryn_protocol_oteryn::ClientCommandView {
                command_type: 9,
                command_id: 1,
                payload: &[8, 2, 18, 0, 48, 1],
            },
            &mut Some(view),
            &mut None,
            selected,
        )
        .await;
        assert!(matches!(
            dispatch,
            Some(Dispatch::ItemMove(ItemMoveOutcome::Rejected, _, _))
        ));
        assert!(owner.move_seen.get().is_none());
        Ok(())
    })
}

struct StoreRetryAuthority {
    inventory: CommandAuthority,
    map_reads: Cell<usize>,
}

impl FreshAdmissionAuthority for StoreRetryAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn observe_character_inventory(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        self.inventory
            .observe_character_inventory(actor, session)
            .await
    }

    async fn observe_house_map(
        &self,
        _: AdmittedSession,
        _: &mut crate::gameplay_transport::world_map::SessionMapView,
        _: &mut SessionItemView,
    ) -> Option<
        Result<
            Option<crate::gameplay_transport::world_map::MapUpdate>,
            crate::gameplay_transport::world_map::MapViewError,
        >,
    > {
        use crate::gameplay_transport::{item_view::ItemViewSnapshotDomain, world_map::MapUpdate};
        let reads = self.map_reads.get();
        self.map_reads.set(reads + 1);
        (reads == 0).then(|| {
            Ok(Some(MapUpdate::Snapshot(ItemViewSnapshotDomain {
                domain_id: oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                revision: 1,
                snapshot_type: 1,
                payload: vec![1],
            })))
        })
    }

    async fn store_command(
        &self,
        _: AdmittedSession,
        _: u64,
        kind: u32,
        payload: Vec<u8>,
    ) -> Option<(CommandStatus, Vec<u8>)> {
        use oteryn_protocol_oteryn::store as wire;
        let reply = match kind {
            wire::COMMAND_TYPE_STORE_INBOX_CLAIM => wire::decode_inbox_claim_request(&payload)
                .and_then(|claim| {
                    wire::encode_claim_receipt(&wire::StoreClaimReceipt {
                        operation_id: [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16],
                        fulfilment_unit_id: claim.fulfilment_unit_id,
                        result: wire::StoreClaimCode::Committed,
                        recovery_generation: 1,
                        decided_at_unix_ms: 1000,
                    })
                }),
            wire::COMMAND_TYPE_STORE_HISTORY_PAGE => Ok(vec![1]),
            _ => return Some((CommandStatus::Rejected, Vec::new())),
        };
        Some(
            reply.map_or((CommandStatus::Rejected, Vec::new()), |bytes| {
                (CommandStatus::Accepted, bytes)
            }),
        )
    }
}

#[test]
fn house_store_claim_retry_keeps_serving_during_map_read_failure() -> Result<(), String> {
    run_command_test(|| async {
        use oteryn_protocol_oteryn::{
            ClientCommandValue, decode_command_result, encode_client_command, store as wire,
        };
        let (admitted, _) = command_fixture(&[
            4,
            oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
            wire::CAPABILITY_STORE_V1,
        ])?;
        let authority = StoreRetryAuthority {
            inventory: CommandAuthority {
                reserved: false,
                move_seen: Cell::new(None),
            },
            map_reads: Cell::new(0),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        let serving = async {
            let mut messages = None;
            let _ = serve(
                &mut server,
                admitted,
                &authority,
                &authority,
                &mut messages,
                IDLE_LIVENESS,
                &mut integrity::Integrity::default(),
                false,
            )
            .await;
            drop(server);
        };
        let driving = async {
            let claim = wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
                fulfilment_unit_id: "00000000-0000-7000-8000-000000000001".into(),
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut received = Vec::new();
            for (command_id, command_type, payload) in [
                (1, wire::COMMAND_TYPE_STORE_INBOX_CLAIM, claim.as_slice()),
                (2, wire::COMMAND_TYPE_STORE_HISTORY_PAGE, &[]),
            ] {
                let frame = encode_client_command(
                    1,
                    &ClientCommandValue {
                        command_id,
                        command_type,
                        payload,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
                // Queue both commands. The committed receipt must not sever House ingress.
                write_frame(&mut client, &frame)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            while let Ok(frame) = read_frame(&mut client).await {
                let envelope = decode_wire_envelope(&frame).map_err(|e| format!("{e:?}"))?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result =
                    decode_command_result(envelope.payload()).map_err(|e| format!("{e:?}"))?;
                assert_eq!(result.status, CommandStatus::Accepted);
                if result.command_id == 1 {
                    assert_eq!(
                        wire::decode_claim_receipt(result.payload)
                            .map_err(|e| format!("{e:?}"))?
                            .result,
                        wire::StoreClaimCode::Committed
                    );
                }
                received.push(result.command_id);
                if received.len() == 2 {
                    break;
                }
            }
            assert_eq!(
                received,
                [1, 2],
                "the next House command must succeed after the committed retry"
            );
            drop(client);
            Ok::<(), String>(())
        };
        let ((), result) = tokio::time::timeout(Duration::from_secs(3), async {
            tokio::join!(serving, driving)
        })
        .await
        .map_err(|e| e.to_string())?;
        result?;
        assert_eq!(authority.map_reads.get(), 1);
        Ok(())
    })
}
