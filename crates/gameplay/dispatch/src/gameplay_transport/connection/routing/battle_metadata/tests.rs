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
    async fn battle_metadata_query(
        &self,
        _: AdmittedSession,
        refs: &[oteryn_protocol_oteryn::world_spatial_entities::EntityRef],
    ) -> Option<Vec<u8>> {
        self.0.set(self.0.get() + 1);
        oteryn_protocol_oteryn::battle_metadata::encode_reply(
            &refs
                .iter()
                .map(
                    |entity| oteryn_protocol_oteryn::battle_metadata::PlayerMetadata {
                        entity: *entity,
                        vocation: Some(4),
                        same_guild: Some(true),
                    },
                )
                .collect::<Vec<_>>(),
        )
    }
}
#[test]
fn battle_metadata_route_enforces_selection_and_bounded_request()
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
                    let payload = oteryn_protocol_oteryn::battle_metadata::encode_query(&[
                        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                            identity: [1; 16],
                            generation: 1,
                        },
                    ])
                    .ok_or("query")?;
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: 740,
                        payload: &payload,
                    };
                    assert!(matches!(
                        route(&authority, &admitted, command).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.0.get(), 0);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: 17,
                            requires: &[],
                        }],
                        &[17],
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
                        oteryn_protocol_oteryn::battle_metadata::decode_reply(&reply)
                            .ok_or("reply")?[0]
                            .vocation,
                        Some(4)
                    );
                    assert_eq!(authority.0.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "battle metadata test thread")?
}
