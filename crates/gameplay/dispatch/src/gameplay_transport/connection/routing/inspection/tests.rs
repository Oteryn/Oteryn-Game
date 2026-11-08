use super::*;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
};
use std::cell::Cell;
struct Authority(Cell<Option<wire::catalogue::Query>>);
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn inspection_command(
        &self,
        _: AdmittedSession,
        _: u64,
        request: Request,
        _: Option<&SessionItemView>,
    ) -> Reply {
        let Request::CataloguePage(query) = request else {
            return Reply::refused(2);
        };
        self.0.set(Some(query));
        Reply {
            status: 1,
            items: vec![],
            requests: vec![],
            catalogue: Some(wire::catalogue::Page {
                next: Some(query.after() + 256),
                categories: vec!["sword".into()],
            }),
        }
    }
}
#[test]
fn catalogue_inspection_route_decodes_filters_and_encodes_continuation()
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
                    let authority = Authority(Cell::new(None));
                    let query =
                        wire::catalogue::Query::new(10, "silver", "sword").map_err(|_| "query")?;
                    let payload = wire::encode_request(Request::CataloguePage(query))
                        .map_err(|_| "request")?;
                    let Dispatch::Reply(CommandStatus::Accepted, bytes) = route(
                        &authority,
                        &mut admitted,
                        ClientCommandView {
                            command_id: 1,
                            command_type: wire::COMMAND_TYPE_INSPECTION,
                            payload: &payload,
                        },
                        &None,
                        &None,
                    )
                    .await
                    else {
                        return Err("catalogue route rejected".into());
                    };
                    let reply = wire::decode_reply(&bytes).map_err(|_| "reply")?;
                    assert_eq!(authority.0.get(), Some(query));
                    assert_eq!(reply.catalogue.and_then(|page| page.next), Some(266));
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "catalogue route thread")?
}
