use super::*;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity},
};
struct Authority(std::cell::Cell<usize>);
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn analyser_party_query(
        &self,
        _: AdmittedSession,
        _: u64,
        prices: &[oteryn_protocol_oteryn::analyser_party::Price],
        _: &mut crate::gameplay_transport::party_command::SessionPartyView,
    ) -> Option<Vec<u8>> {
        assert!(prices.is_empty());
        self.0.set(self.0.get() + 1);
        oteryn_protocol_oteryn::analyser_party::encode_reply(&[]).ok()
    }
}
#[test]
fn party_accounting_route_requires_both_capabilities_and_valid_prices()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            tokio::runtime::Builder::new_current_thread()
                .build()?
                .block_on(async {
                    let (runtime, actor, session) =
                        crate::gameplay_transport::actor_spell::tests::runtime_with_player(31);
                    let mut admitted = AdmittedSession {
                        game_session_id: session,
                        world_id: runtime.binding().world_id(),
                        channel_id: runtime.binding().channel_id().ok_or("channel")?,
                        runtime_actor: Some(actor),
                        first_entry: FirstEntryOutcome::Positioned,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    let authority = Authority(std::cell::Cell::new(0));
                    let mut view =
                        crate::gameplay_transport::party_command::SessionPartyView::default();
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: 735,
                        payload: &[0, 0],
                    };
                    let offered = [
                        OfferedCapability {
                            id: 10,
                            requires: &[],
                        },
                        OfferedCapability {
                            id: 42,
                            requires: &[],
                        },
                    ];
                    for selected in [&[][..], &[10][..], &[42][..]] {
                        admitted.continuity.selected_capabilities =
                            SelectedCapabilities::select(&offered, selected).ok_or("selection")?;
                        assert!(matches!(
                            route(&authority, &admitted, command, &mut view).await,
                            Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                        ));
                    }
                    assert_eq!(authority.0.get(), 0);
                    admitted.continuity.selected_capabilities =
                        SelectedCapabilities::select(&offered, &[10, 42]).ok_or("selection")?;
                    assert!(matches!(
                        route(
                            &authority,
                            &admitted,
                            oteryn_protocol_oteryn::ClientCommandView {
                                payload: &[0],
                                ..command
                            },
                            &mut view
                        )
                        .await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.0.get(), 0);
                    assert!(matches!(
                        route(&authority, &admitted, command, &mut view).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Accepted, _)
                    ));
                    assert_eq!(authority.0.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "party route thread")?
}
