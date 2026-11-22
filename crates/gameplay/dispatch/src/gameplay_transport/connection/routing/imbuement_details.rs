use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, imbuement, imbuement_details as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
    if command.command_type == wire::COMMAND_TYPE_IMBUEMENT_PROTECTION {
        let selected = admitted.continuity.selected_capabilities;
        let payload = if selected.contains(wire::CAPABILITY_IMBUEMENT_AFFORDABILITY_V1)
            && selected.contains(wire::CAPABILITY_IMBUEMENT_DETAILS_V1)
            && selected.contains(imbuement::CAPABILITY_IMBUE_V1)
            && imbuement::decode_imbuement_request(command.payload).is_ok_and(|request| {
                matches!(request.action, imbuement::ImbuementAction::Apply { .. })
            })
        {
            wire::IMBUEMENT_PROTECTION_REFUSAL.as_bytes().to_vec()
        } else {
            Vec::new()
        };
        return Some(Dispatch::Reply(CommandStatus::Rejected, payload));
    }
    match transport::imbuement_details::query(authority, admitted, command, item_view.as_ref())
        .await
    {
        transport::imbuement::ImbuementReply::Payload(bytes) => {
            Some(Dispatch::Reply(CommandStatus::Accepted, bytes))
        }
        transport::imbuement::ImbuementReply::Rejected => {
            Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()))
        }
        transport::imbuement::ImbuementReply::Unavailable => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, GameSessionId, WorldId};
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use crate::gameplay_transport::{
        AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
    };
    use oteryn_protocol_oteryn::item_view::ItemHandle;

    struct Owner;
    impl FreshAdmissionAuthority for Owner {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
    }

    #[tokio::test]
    async fn imbuement_protection_route_refuses_the_purchase() -> Result<(), String> {
        let id = [1, 0x90, 0, 0, 0, 9, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 9];
        let mut admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?,
            world_id: WorldId::decode(&id).map_err(|e| format!("{e:?}"))?,
            channel_id: ChannelId::decode(&id).map_err(|e| format!("{e:?}"))?,
            runtime_actor: None,
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let payload = imbuement::encode_imbuement_request(&imbuement::ImbuementRequest {
            item: ItemHandle::new(9).ok_or("item handle")?,
            action: imbuement::ImbuementAction::Apply { slot: 0, imbuement: 0 },
        }).map_err(|e| format!("{e:?}"))?;
        let command = ClientCommandView {
            command_id: 1,
            command_type: wire::COMMAND_TYPE_IMBUEMENT_PROTECTION,
            payload: &payload,
        };
        assert!(matches!(route(&Owner, &admitted, command, &None).await,
            Some(Dispatch::Reply(CommandStatus::Rejected, bytes)) if bytes.is_empty()));
        let offered = [
            OfferedCapability { id: 4, requires: &[] },
            OfferedCapability { id: 68, requires: &[4] },
            OfferedCapability { id: 447, requires: &[68] },
            OfferedCapability { id: 582, requires: &[447] },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[4, 68, 447, 582]).ok_or("imbuement selection")?;
        assert!(matches!(route(&Owner, &admitted, command, &None).await,
            Some(Dispatch::Reply(CommandStatus::Rejected, bytes)) if bytes == b"Sorry, not possible."));
        let malformed = ClientCommandView { payload: &[], ..command };
        assert!(matches!(route(&Owner, &admitted, malformed, &None).await,
            Some(Dispatch::Reply(CommandStatus::Rejected, bytes)) if bytes.is_empty()));
        Ok(())
    }
}
