//! INVCAP-1: this connection's own item commit or a changed maximum health or mana opens the
//! bounded capacity refresh, so `Cap` follows the player's own action on the next tick.
use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;
use crate::gameplay_transport::item_view::{InventoryItems, ViewItem};
use oteryn_protocol_oteryn::hotkey_equip::{COMMAND_TYPE_HOTKEY_EQUIP, Request};
use oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity;
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
use std::time::Duration;

struct RereadAuthority {
    reads: Cell<usize>,
    levelled: Cell<bool>,
}

impl FreshAdmissionAuthority for RereadAuthority {
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

    async fn observe_vitals(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        Some(if self.levelled.get() {
            (
                2,
                ActorVitals {
                    max_health: SERENE_VITALS.max_health + 5,
                    max_mana: SERENE_VITALS.max_mana + 5,
                    ..SERENE_VITALS
                },
            )
        } else {
            (1, SERENE_VITALS)
        })
    }

    async fn hotkey_equip_wait(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: Request,
        _: Duration,
    ) -> Option<Duration> {
        Some(Duration::ZERO)
    }

    async fn hotkey_equip(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: Request,
    ) -> Option<ItemMoveOutcome> {
        Some(ItemMoveOutcome::Moved)
    }

    async fn committed_equipment_move(
        &self,
        _: ExactActorRef,
        _: UseCommand,
    ) -> Option<EquipmentMoveReplay> {
        Some(EquipmentMoveReplay::Absent)
    }

    async fn committed_loop_move(&self, _: ExactActorRef, _: UseCommand) -> Option<LoopMoveReplay> {
        Some(LoopMoveReplay::Absent)
    }

    async fn stash(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        intent: oteryn_protocol_oteryn::stash::StashIntent,
        _: Option<crate::gameplay_transport::item_view::ItemKey>,
        _: bool,
    ) -> Option<oteryn_protocol_oteryn::stash::StashReply> {
        use oteryn_protocol_oteryn::stash::{StashAction, StashOutcome, StashReply};
        assert!(matches!(intent.action, StashAction::Stow(_)));
        Some(StashReply {
            outcome: StashOutcome::Committed,
            stowed_quantity: 1,
            transaction: Some([1; 16]),
            page: 0,
            has_more: false,
            rows: Vec::new(),
        })
    }

    async fn observe_inventory_capacity(&self, _: AdmittedSession) -> Option<InventoryCapacity> {
        self.reads.set(self.reads.get() + 1);
        Some(InventoryCapacity {
            capacity: 40_000,
            free: 39_000,
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

/// Joins, waits past the first read, runs `act` and expects `reads` reads `wait` later; the
/// unchanged 5 s bound would allow no second read inside that window.
fn reread_case(
    act: fn(&RereadAuthority) -> Option<Vec<u8>>,
    wait: Duration,
    reads: usize,
    message: &'static str,
) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move {
                tokio::time::pause();
                let authority = RereadAuthority {
                    reads: Cell::new(0),
                    levelled: Cell::new(false),
                };
                let ids = [4, 6, 12, 14, 62, 179, 508];
                let offered = ids.map(|id| OfferedCapability { id, requires: &[] });
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(&offered, &ids).ok_or("capabilities")?;
                let (mut server, mut client) = tokio::io::duplex(256 * 1024);
                let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
                let driving = async {
                    loop {
                        let frame =
                            crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
                        if decode_wire_envelope(&frame)?.message_type()
                            == MessageType::SnapshotCommit
                        {
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(1100)).await;
                    assert_eq!(authority.reads.get(), 1, "the first tick reads once");
                    if let Some(frame) = act(&authority) {
                        client.write_all(&framed(&frame)).await?;
                    }
                    let start = tokio::time::Instant::now();
                    while tokio::time::Instant::now() - start < wait {
                        // Drain frames so the server never blocks on a full duplex.
                        let _ = tokio::time::timeout(
                            Duration::from_millis(50),
                            crate::gameplay_transport::tcp_tls::read_frame(&mut client),
                        )
                        .await;
                    }
                    assert_eq!(authority.reads.get(), reads, "{message}");
                    client.shutdown().await?;
                    Ok::<_, Box<dyn Error>>(())
                };
                let (end, result) = tokio::join!(serving, driving);
                result?;
                assert!(
                    matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                    "{end:?}"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "capacity reread connection test panicked")?
        .map_err(Into::into)
}

#[test]
fn inventory_capacity_rereads_after_own_item_move() -> Result<(), Box<dyn Error>> {
    reread_case(
        |_| Some(equip_frame(1, 1)),
        Duration::from_millis(600),
        2,
        "an own committed item move rereads on the next tick",
    )
}

#[test]
fn inventory_capacity_rereads_after_max_vitals_change() -> Result<(), Box<dyn Error>> {
    reread_case(
        |authority| {
            authority.levelled.set(true);
            None
        },
        Duration::from_millis(1600),
        2,
        "a changed maximum health or mana rereads",
    )
}

#[test]
fn inventory_capacity_keeps_bound_without_own_change() -> Result<(), Box<dyn Error>> {
    reread_case(
        |_| None,
        Duration::from_millis(1600),
        1,
        "no own change keeps the 5 s bound",
    )
}

#[test]
fn inventory_capacity_rereads_after_stash_commit() -> Result<(), Box<dyn Error>> {
    reread_case(
        |_| {
            use oteryn_protocol_oteryn::stash::{
                COMMAND_TYPE_STASH_INTENT, StashAction, StashIntent, encode_stash_intent,
            };
            let intent = encode_stash_intent(StashIntent {
                action: StashAction::Stow(std::num::NonZeroU64::new(1)?),
                page: 0,
            })
            .ok()?;
            let mut payload = Vec::new();
            scalar(&mut payload, 1, 1);
            scalar(&mut payload, 2, u64::from(COMMAND_TYPE_STASH_INTENT));
            bytes(&mut payload, 4, &intent);
            Some(envelope(7, 1, &payload))
        },
        Duration::from_millis(600),
        2,
        "a committed stow rereads capacity on the next tick",
    )
}
