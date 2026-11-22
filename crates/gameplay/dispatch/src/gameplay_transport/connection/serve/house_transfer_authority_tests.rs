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
    ) -> Option<Vec<u8>> {
        assert_eq!(
            item,
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(
                [2; 16]
            ))
        );
        oteryn_protocol_oteryn::item_inscription::Reply {
            text: "Resident letter".into(),
            max_length: 1999,
            writable: true,
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

#[derive(Default)]
struct PendingPodiumAuthority {
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
    query_ready: tokio::sync::Notify,
    periodic_ready: tokio::sync::Notify,
    periodic_continue: tokio::sync::Notify,
    fail_periodic_write: bool,
}

impl FreshAdmissionAuthority for PendingPodiumAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn actor_step_deadline(&self, _: ExactActorRef) -> Option<Instant> {
        Some(Instant::now() + Duration::from_secs(30))
    }

    async fn house_podium_approach(&self, _: AdmittedSession, _: [u8; 16]) -> Approach {
        self.query_ready.notify_one();
        Approach::Walk(vec![StepDirection::East])
    }

    async fn observe_house_vitals(
        &self,
        _: ExactActorRef,
        _: AdmittedSession,
    ) -> Option<(u64, ActorVitals)> {
        if !self.fail_periodic_write {
            return None;
        }
        self.periodic_ready.notify_one();
        self.periodic_continue.notified().await;
        Some((
            2,
            ActorVitals {
                health: 184,
                max_health: 185,
                mana: 90,
                max_mana: 90,
                soul: 100,
                harmony: 2,
                serene: true,
            },
        ))
    }
}

#[test]
fn house_podium_reservation_allows_reconnect_after_read_or_periodic_write_failure()
-> Result<(), String> {
    run_command_test(|| async {
        tokio::time::pause();
        for fail_periodic_write in [false, true] {
            let owner = PendingPodiumAuthority {
                fail_periodic_write,
                ..Default::default()
            };
            let (mut admitted, _) = command_fixture(&[49, 150, 508])?;
            admitted.continuity.vitals_revision = 1;
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .domain_selected(STATE_DOMAIN_ACTOR_VITALS)
            );
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            let query = oteryn_protocol_oteryn::encode_client_command(
                admitted.continuity.connection_generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_QUERY,
                    payload: &[1; 16],
                },
            )
            .map_err(|e| format!("{e:?}"))?;
            write_frame(&mut client, &query)
                .await
                .map_err(|e| e.to_string())?;
            let client = async {
                owner.query_ready.notified().await;
                if fail_periodic_write {
                    owner.periodic_ready.notified().await;
                }
                drop(client);
                owner.periodic_continue.notify_one();
            };
            let mut messages = None;
            let mut integrity = integrity::Integrity::default();
            let (result, ()) = tokio::time::timeout(Duration::from_secs(4), async {
                tokio::join!(
                    serve(
                        &mut server,
                        admitted,
                        &owner,
                        &owner,
                        &mut messages,
                        IDLE_LIVENESS,
                        &mut integrity,
                        false,
                    ),
                    client
                )
            })
            .await
            .map_err(|e| e.to_string())?;
            let Err(end) = result else {
                return Err("expected House connection loss".into());
            };
            let ConnectionEnd::AdmittedThenDisconnected(mut ended) = *end else {
                return Err("unexpected House connection end".into());
            };
            assert_eq!(ended.continuity.next_command_id, 2);
            assert!(owner.commands.resumable(&ended));
            ended.continuity.connection_generation += 1;
            assert!(
                crate::gameplay_transport::session_ingress::SessionCommandStream::open(
                    Some(&owner.commands),
                    &ended,
                )
                .is_some()
            );
        }
        Ok(())
    })
}
