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
    async fn house_podium(
        &self,
        _: AdmittedSession,
        request: crate::gameplay_transport::house_podium::Request,
    ) -> Option<Vec<u8>> {
        let crate::gameplay_transport::house_podium::Request::Preview(request) = request else {
            return None;
        };
        self.0.set(self.0.get() + 1);
        assert_eq!(request.key, "oteryn:creature.rat");
        oteryn_protocol_oteryn::house_podium::preview::Artwork::encode(Some(
            oteryn_protocol_oteryn::house_podium::preview::Artwork {
                object: false,
                direction: request.direction,
                looks: [21; 2],
            },
        ))
    }
}
#[test]
fn podium_artwork_route_decodes_draft_and_returns_real_artwork_codec()
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
                    let payload = oteryn_protocol_oteryn::house_podium::preview::Request {
                        display_kind: 3,
                        direction: 1,
                        key: "oteryn:creature.rat".into(),
                    }
                    .encode()
                    .ok_or("request")?;
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: 150,
                            requires: &[],
                        }],
                        &[150],
                    )
                    .ok_or("selection")?;
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: 739,
                        payload: &payload,
                    };
                    assert!(
                        route(
                            &authority,
                            &admitted,
                            oteryn_protocol_oteryn::ClientCommandView {
                                payload: &[3, 1, 160],
                                ..command
                            }
                        )
                        .await
                        .is_none()
                    );
                    assert_eq!(authority.0.get(), 0);
                    let Some(Dispatch::Reply(crate::foundation::CommandStatus::Accepted, reply)) =
                        route(&authority, &admitted, command).await
                    else {
                        return Err("artwork reply".into());
                    };
                    assert_eq!(
                        oteryn_protocol_oteryn::house_podium::preview::Artwork::decode(&reply)
                            .flatten()
                            .ok_or("artwork")?
                            .looks,
                        [21, 21]
                    );
                    assert_eq!(authority.0.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "podium artwork test thread")?
}
