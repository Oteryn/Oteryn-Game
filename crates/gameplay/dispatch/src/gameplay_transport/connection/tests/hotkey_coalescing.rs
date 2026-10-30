use super::*;
use crate::gameplay_transport::item_view::{InventoryItems, ViewItem};
use oteryn_protocol_oteryn::hotkey_equip::{COMMAND_TYPE_HOTKEY_EQUIP, Request};
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
use std::time::Duration;

type EquipObservation = (u64, u32, tokio::time::Instant);
type EquipResult = (u64, ItemMoveOutcome);

struct EquipAuthority {
    due: Cell<tokio::time::Instant>,
    equipped: RefCell<Vec<EquipObservation>>,
    committed_first: bool,
}

impl FreshAdmissionAuthority for EquipAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems::default())
    }

    async fn observe_character_equipment(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<Vec<(EquipmentSlot, ViewItem)>> {
        Some(Vec::new())
    }

    async fn hotkey_equip_wait(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        request: Request,
    ) -> Option<Duration> {
        // Reference 7 stands for jewelry; 99 is an unknown definition.
        (!matches!(request.definition.get(), 7 | 99)).then(|| {
            self.due
                .get()
                .saturating_duration_since(tokio::time::Instant::now())
        })
    }

    async fn hotkey_equip(
        &self,
        _: ExactActorRef,
        command: UseCommand,
        request: Request,
    ) -> Option<ItemMoveOutcome> {
        if self.committed_first && command.command_id == 1 {
            return Some(ItemMoveOutcome::Moved);
        }
        tokio::time::sleep_until(self.due.get()).await;
        let now = tokio::time::Instant::now();
        self.equipped
            .borrow_mut()
            .push((command.command_id, request.definition.get(), now));
        self.due.set(now + Duration::from_millis(200));
        Some(ItemMoveOutcome::Moved)
    }

    async fn committed_equipment_move(
        &self,
        _: ExactActorRef,
        command: UseCommand,
    ) -> Option<EquipmentMoveReplay> {
        Some(if self.committed_first && command.command_id == 1 {
            EquipmentMoveReplay::Moved
        } else {
            EquipmentMoveReplay::Absent
        })
    }
}

fn equip_frame(id: u64, definition: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(&mut payload, 2, u64::from(COMMAND_TYPE_HOTKEY_EQUIP));
    bytes(&mut payload, 4, &definition.to_be_bytes());
    envelope(7, 1, &payload)
}

#[test]
fn hotkey_equip_coalesces_burst_to_latest_command_with_ordered_results()
-> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let start = tokio::time::Instant::now();
                let authority = EquipAuthority {
                    due: Cell::new(start + Duration::from_millis(200)),
                    equipped: RefCell::new(Vec::new()),
                    committed_first: false,
                };
                let offered = [4, 12, 14, 179].map(|id| OfferedCapability { id, requires: &[] });
                let admitted = AdmittedSession {
                    continuity: SessionContinuity {
                        selected_capabilities: SelectedCapabilities::select(
                            &offered,
                            &[4, 12, 14, 179],
                        )
                        .ok_or("selection")?,
                        ..SessionContinuity::FRESH
                    },
                    ..positioned()?
                };
                let (_, frames) = drive_session(
                    &authority,
                    admitted,
                    &[equip_frame(1, 1), equip_frame(2, 2), equip_frame(3, 3)],
                )
                .await?;
                let equipped = authority.equipped.borrow();
                assert_eq!(
                    equipped
                        .iter()
                        .map(|(id, definition, _)| (*id, *definition))
                        .collect::<Vec<_>>(),
                    [(3, 3)]
                );
                assert!(equipped[0].2 - start >= Duration::from_millis(200));
                let mut results = Vec::new();
                for frame in &frames {
                    let envelope = decode_wire_envelope(frame)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                        results.push((
                            result.command_id,
                            oteryn_protocol_oteryn::hotkey_equip::decode_reply(result.payload)
                                .map_err(|e| format!("{e:?}"))?,
                        ));
                    }
                }
                assert_eq!(
                    results,
                    [
                        (1, ItemMoveOutcome::Rejected),
                        (2, ItemMoveOutcome::Rejected),
                        (3, ItemMoveOutcome::Moved)
                    ]
                );
                Ok(())
            })
            .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "scenario panicked")?
        .map_err(Into::into)
}

#[test]
fn hotkey_equip_coalescing_preserves_cooldown_after_each_executed_press()
-> Result<(), Box<dyn Error>> {
    equip_case(
        vec![equip_frame(1, 1), equip_frame(2, 2), equip_frame(3, 3)],
        false,
        false,
        |equipped, results| {
            assert_eq!(
                equipped.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                [1, 3]
            );
            assert!(equipped[1].2 - equipped[0].2 >= Duration::from_millis(200));
            assert_eq!(
                results,
                [
                    (1, ItemMoveOutcome::Moved),
                    (2, ItemMoveOutcome::Rejected),
                    (3, ItemMoveOutcome::Moved)
                ]
            );
        },
    )
}

#[test]
fn hotkey_equip_coalescing_does_not_replace_with_jewelry_unknown_or_malformed_input()
-> Result<(), Box<dyn Error>> {
    for definition in [0, 7, 99] {
        equip_case(
            vec![equip_frame(1, 1), equip_frame(2, definition)],
            true,
            false,
            |equipped, results| {
                assert_eq!(equipped.first().map(|(id, _, _)| *id), Some(1));
                assert_eq!(results.first(), Some(&(1, ItemMoveOutcome::Moved)));
            },
        )?;
    }
    Ok(())
}

#[test]
fn hotkey_equip_coalescing_keeps_committed_replay_and_rejects_sequence_or_generation_gaps()
-> Result<(), Box<dyn Error>> {
    equip_case(
        vec![equip_frame(1, 1), equip_frame(2, 2)],
        true,
        true,
        |equipped, results| {
            assert_eq!(
                equipped.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                [2]
            );
            assert_eq!(
                results,
                [(1, ItemMoveOutcome::Moved), (2, ItemMoveOutcome::Moved)]
            );
        },
    )?;
    let mut stale_payload = Vec::new();
    scalar(&mut stale_payload, 1, 2);
    scalar(&mut stale_payload, 2, u64::from(COMMAND_TYPE_HOTKEY_EQUIP));
    bytes(&mut stale_payload, 4, &2u32.to_be_bytes());
    for next in [equip_frame(3, 2), envelope(7, 2, &stale_payload)] {
        equip_case(
            vec![equip_frame(1, 1), next],
            true,
            false,
            |equipped, results| {
                assert_eq!(
                    equipped.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                    [1]
                );
                assert_eq!(results, [(1, ItemMoveOutcome::Moved)]);
            },
        )?;
    }
    Ok(())
}

fn equip_case(
    inputs: Vec<Vec<u8>>,
    delayed: bool,
    committed_first: bool,
    check: fn(&[EquipObservation], &[EquipResult]),
) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async {
                tokio::time::pause();
                let authority = EquipAuthority {
                    due: Cell::new(
                        tokio::time::Instant::now()
                            + Duration::from_millis(if delayed { 200 } else { 0 }),
                    ),
                    equipped: RefCell::new(Vec::new()),
                    committed_first,
                };
                let offered = [4, 12, 14, 179].map(|id| OfferedCapability { id, requires: &[] });
                let admitted = AdmittedSession {
                    continuity: SessionContinuity {
                        selected_capabilities: SelectedCapabilities::select(
                            &offered,
                            &[4, 12, 14, 179],
                        )
                        .ok_or("selection")?,
                        ..SessionContinuity::FRESH
                    },
                    ..positioned()?
                };
                let (_, frames) = drive_session(&authority, admitted, &inputs).await?;
                let mut results = Vec::new();
                for frame in &frames {
                    let envelope = decode_wire_envelope(frame)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                        results.push((
                            result.command_id,
                            oteryn_protocol_oteryn::hotkey_equip::decode_reply(result.payload)
                                .map_err(|e| format!("{e:?}"))?,
                        ));
                    }
                }
                check(&authority.equipped.borrow(), &results);
                Ok(())
            })
            .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "scenario panicked")?
        .map_err(Into::into)
}
