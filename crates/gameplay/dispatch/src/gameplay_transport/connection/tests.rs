// Included topic files share this module so test paths and private access stay stable.
#[path = "tests/buffered_step_recovery.rs"]
mod buffered_step_recovery;

#[path = "tests/capability_tests.rs"]
mod capability_tests;

#[path = "tests/guard_deferral.rs"]
mod guard_deferral;

#[path = "tests/hotkey_coalescing.rs"]
mod hotkey_coalescing;

#[path = "tests/idle_kick.rs"]
mod idle_kick;

#[path = "tests/integrity.rs"]
mod integrity;

#[path = "tests/inventory_capacity_reread.rs"]
mod inventory_capacity_reread;

#[path = "tests/sidecar_header.rs"]
mod sidecar_header;

mod bed_sleep_dispatch_tests {
    include!("../bed_sleep_dispatch_tests.rs");
}

mod item_transform_dispatch_tests {
    include!("../item_transform_dispatch_tests.rs");
}

#[path = "tests/write_text_depot_view.rs"]
mod write_text_depot_view;

use super::*;
use crate::gameplay_transport::world_spatial::encode_step_result;
use oteryn_protocol_oteryn::world_object::decode_use_result_with_gate;
use std::cell::{Cell, RefCell};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

#[path = "tests/support.rs"]
mod support;

#[path = "tests/spell_catalog.rs"]
mod spell_catalog;
use support::*;

include!("../step_dispatch_fixture.rs");
include!("../tutorial_session_tests.rs");
include!("../newhaven_travel_transport_tests.rs");
include!("../spectate_session_ingress_tests.rs");

include!("tests/use_capabilities.rs");
include!("tests/pending_spell.rs");
include!("tests/bank_resume.rs");
include!("tests/liveness.rs");
include!("tests/steps_pacing.rs");
include!("tests/admission.rs");
include!("tests/overlay.rs");
include!("tests/use_commit.rs");
include!("tests/spell_vitals.rs");
include!("tests/spell_resume.rs");
include!("tests/achievements_highscores.rs");
include!("tests/notices.rs");

#[test]
fn inventory_capacity_observation_is_bounded_through_connection() -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                use crate::gameplay_transport::capabilities::OfferedCapability;
                use oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity;

                struct CapacityAuthority(Cell<usize>);
                impl FreshAdmissionAuthority for CapacityAuthority {
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
                    async fn observe_inventory_capacity(
                        &self,
                        _: AdmittedSession,
                    ) -> Option<InventoryCapacity> {
                        self.0.set(self.0.get() + 1);
                        Some(InventoryCapacity {
                            capacity: 40_000,
                            free: 39_000,
                        })
                    }
                }
                tokio::time::pause();
                let authority = CapacityAuthority(Cell::new(0));
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    &[
                        OfferedCapability {
                            id: 4,
                            requires: &[6],
                        },
                        OfferedCapability {
                            id: 6,
                            requires: &[],
                        },
                        OfferedCapability {
                            id: 508,
                            requires: &[4, 6],
                        },
                    ],
                    &[4, 6, 508],
                )
                .ok_or("capacity capabilities")?;
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
                    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
                    assert_eq!(
                        authority.0.get(),
                        1,
                        "an unchanged inventory must not be reread on visibility ticks"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    assert_eq!(
                        authority.0.get(),
                        2,
                        "a bounded slower refresh must still observe external changes"
                    );
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
        .map_err(|_| "capacity observation connection test panicked")?
        .map_err(Into::into)
}

#[tokio::test]
async fn malformed_frame_yields_protocol_error_then_close() -> Result<(), Box<dyn Error>> {
    let (mut client, mut server) = tokio::io::duplex(256);
    let admitted = positioned()?;
    let task = tokio::spawn(async move { hold_admitted(&mut server, admitted).await });

    // A zero body length is forbidden by FND-02 framing.
    client.write_all(&0_u32.to_be_bytes()).await?;
    client.shutdown().await?;
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let frames = split_frames(&output)?;
    assert_eq!(frames.len(), 1);
    let envelope = decode_wire_envelope(&frames[0])?;
    assert_eq!(envelope.message_type(), MessageType::ProtocolError);
    assert_eq!(
        oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())?.error_code,
        FoundationProtocolError::MalformedFrame.code()
    );
    assert!(matches!(
        task.await?,
        ConnectionEnd::AdmittedThenClosed(_, FoundationProtocolError::MalformedFrame)
    ));
    Ok(())
}
