use super::*;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity},
};
use std::cell::Cell;
struct Authority(Cell<usize>);
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn analyser_catalogue_query(&self, _: AdmittedSession, ids: &[u32]) -> Option<Vec<u8>> {
        self.0.set(self.0.get() + 1);
        oteryn_protocol_oteryn::analyser_catalogue::encode_reply(
            &ids.iter()
                .map(|id| oteryn_protocol_oteryn::analyser_catalogue::ItemInfo {
                    item_type: *id,
                    name: "Item".into(),
                    coin_worth: 0,
                })
                .collect::<Vec<_>>(),
        )
        .ok()
    }
}
#[test]
fn analyser_catalogue_route_enforces_selection_and_bounded_request()
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
                    let authority = Authority(Cell::new(0));
                    let payload = oteryn_protocol_oteryn::analyser_catalogue::encode_request(&[1])?;
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: 734,
                        payload: &payload,
                    };
                    assert!(matches!(
                        route(&authority, &admitted, command).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.0.get(), 0);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: 10,
                            requires: &[],
                        }],
                        &[10],
                    )
                    .ok_or("selection")?;
                    assert!(matches!(
                        route(
                            &authority,
                            &admitted,
                            oteryn_protocol_oteryn::ClientCommandView {
                                payload: &[1, 0],
                                ..command
                            }
                        )
                        .await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.0.get(), 0);
                    let Dispatch::Reply(crate::foundation::CommandStatus::Accepted, reply) =
                        route(&authority, &admitted, command).await
                    else {
                        return Err("reply".into());
                    };
                    assert_eq!(
                        oteryn_protocol_oteryn::analyser_catalogue::decode_reply(&reply)?[0]
                            .item_type,
                        1
                    );
                    assert_eq!(authority.0.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "analyser catalogue test thread")?
}
