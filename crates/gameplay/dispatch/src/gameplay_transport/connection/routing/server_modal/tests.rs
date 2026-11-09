use super::*;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity},
    raid_notices::ServerMessageHub,
    server_modal::ModalHub,
};
use oteryn_protocol_oteryn::server_modal::{
    Answer, CAPABILITY_SERVER_MODAL_V1, COMMAND_TYPE_SERVER_MODAL_ANSWER,
    COMMAND_TYPE_SERVER_MODAL_QUERY, Dialog, Image, Reply,
};
use std::cell::Cell;
struct Authority {
    notices: ServerMessageHub,
    modals: ModalHub,
    calls: Cell<u32>,
}
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn server_modal_execute(
        &self,
        admitted: AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        self.calls.set(self.calls.get() + 1);
        let root = self.notices.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        self.modals.execute(
            root,
            kind,
            payload,
            Some(Dialog {
                id: 1,
                title: "Server choice".into(),
                message: "Choose an entry".into(),
                choices: vec![(7, "First".into())],
                buttons: vec![(1, "Accept".into()), (2, "Cancel".into())],
                cancel_button: Some(2),
                image: Some(Image {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                }),
            }),
        )
    }
}
#[test]
fn server_modal_real_route_checks_capability_connection_ids_and_duplicate_answers()
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
                    admitted.continuity.connection_generation = 1;
                    let authority = Authority {
                        notices: Default::default(),
                        modals: Default::default(),
                        calls: Cell::new(0),
                    };
                    assert!(authority.notices.join(session, 1));
                    let query = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: COMMAND_TYPE_SERVER_MODAL_QUERY,
                        payload: &[],
                    };
                    assert!(matches!(
                        route(&authority, &admitted, query).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.calls.get(), 0);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[
                            OfferedCapability {
                                id: 92,
                                requires: &[],
                            },
                            OfferedCapability {
                                id: CAPABILITY_SERVER_MODAL_V1,
                                requires: &[92],
                            },
                        ],
                        &[92, CAPABILITY_SERVER_MODAL_V1],
                    )
                    .ok_or("selection")?;
                    let Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) =
                        route(&authority, &admitted, query).await
                    else {
                        return Err("query reply".into());
                    };
                    let Reply::Dialog(dialog) = Reply::decode(&payload)? else {
                        return Err("dialog".into());
                    };
                    assert_eq!(
                        dialog.image.as_ref().map(|image| image.rgba.as_slice()),
                        Some([255, 0, 0, 255].as_slice())
                    );
                    let answer = Answer {
                        dialog_id: dialog.id,
                        button_id: 1,
                        choice_id: Some(7),
                    };
                    let invalid = Answer {
                        choice_id: Some(99),
                        ..answer
                    }
                    .encode()?;
                    assert!(matches!(
                        route(
                            &authority,
                            &admitted,
                            oteryn_protocol_oteryn::ClientCommandView {
                                command_type: COMMAND_TYPE_SERVER_MODAL_ANSWER,
                                payload: &invalid,
                                ..query
                            }
                        )
                        .await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    let bytes = answer.encode()?;
                    for _ in 0..2 {
                        let Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) =
                            route(
                                &authority,
                                &admitted,
                                oteryn_protocol_oteryn::ClientCommandView {
                                    command_type: COMMAND_TYPE_SERVER_MODAL_ANSWER,
                                    payload: &bytes,
                                    ..query
                                },
                            )
                            .await
                        else {
                            return Err("answer reply".into());
                        };
                        assert_eq!(Reply::decode(&payload)?, Reply::Answered(answer));
                    }
                    let Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) =
                        route(&authority, &admitted, query).await
                    else {
                        return Err("empty reply".into());
                    };
                    assert_eq!(Reply::decode(&payload)?, Reply::None);
                    admitted.continuity.connection_generation = 2;
                    assert!(matches!(
                        route(&authority, &admitted, query).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert!(authority.notices.join(session, 2));
                    assert!(matches!(
                        route(
                            &authority,
                            &admitted,
                            oteryn_protocol_oteryn::ClientCommandView {
                                command_type: COMMAND_TYPE_SERVER_MODAL_ANSWER,
                                payload: &bytes,
                                ..query
                            }
                        )
                        .await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "server modal route thread failed")?
}
