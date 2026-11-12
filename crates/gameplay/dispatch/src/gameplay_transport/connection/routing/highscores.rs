//! Highscores command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, FreshAdmissionAuthority, HighscoresReply, decode_highscores_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::highscores_loyalty::{
    CAPABILITY_HIGHSCORES_LOYALTY_V1, LOYALTY_CATEGORY,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    // HIGHSCORES0 §6: the World and the asking Character are the admitted session's.
    match decode_highscores_query(command.payload) {
        // highscores-loyalty-v1.md: Loyalty stays NOT_SUPPORTED unless capability 498 is selected.
        Ok(query)
            if query.category == LOYALTY_CATEGORY
                && !admitted
                    .continuity
                    .selected_capabilities
                    .contains(CAPABILITY_HIGHSCORES_LOYALTY_V1) =>
        {
            Dispatch::Highscores(HighscoresReply::Terminal(
                crate::foundation::FoundationProtocolError::NotSupported,
            ))
        }
        Ok(query) => Dispatch::Highscores(
            authority
                .highscores(admitted.world_id, admitted.game_session_id, query)
                .await,
        ),
        Err(_) => Dispatch::Highscores(HighscoresReply::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, FoundationProtocolError, GameSessionId, WorldId};
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use crate::gameplay_transport::{
        AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
    };
    use oteryn_protocol_oteryn::highscores::{
        COMMAND_TYPE_HIGHSCORES_QUERY_V1, HighscoresQuery, encode_highscores_query,
    };

    /// Accepts every query it is asked with a result naming the asked category.
    struct Owner;
    impl FreshAdmissionAuthority for Owner {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn highscores(
            &self,
            _world: WorldId,
            _session: GameSessionId,
            query: HighscoresQuery,
        ) -> HighscoresReply {
            HighscoresReply::Result(query.category.to_le_bytes().to_vec())
        }
    }

    fn uuid(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    fn accepted(dispatch: Dispatch, category: u32) -> bool {
        matches!(
            dispatch,
            Dispatch::Highscores(HighscoresReply::Result(bytes))
                if bytes == category.to_le_bytes()
        )
    }

    #[test]
    fn loyalty_category_is_not_supported_without_capability_498() -> Result<(), String> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let mut admitted = AdmittedSession {
                    game_session_id: GameSessionId::decode(&uuid(0x62))
                        .map_err(|e| format!("{e:?}"))?,
                    world_id: WorldId::decode(&uuid(0x60)).map_err(|e| format!("{e:?}"))?,
                    channel_id: ChannelId::decode(&uuid(0x61)).map_err(|e| format!("{e:?}"))?,
                    runtime_actor: None,
                    first_entry: FirstEntryOutcome::Positioned,
                    controller: None,
                    continuity: SessionContinuity::FRESH,
                    item_fence: None,
                };
                let offered = [
                    OfferedCapability {
                        id: 9,
                        requires: &[],
                    },
                    OfferedCapability {
                        id: CAPABILITY_HIGHSCORES_LOYALTY_V1,
                        requires: &[9],
                    },
                ];
                let payload = |category| {
                    encode_highscores_query(HighscoresQuery {
                        world: None,
                        details: false,
                        category,
                        vocation: 0,
                        page: 1,
                        snapshot_id: 0,
                    })
                    .map_err(|e| format!("{e:?}"))
                };
                let command = |payload| ClientCommandView {
                    command_id: 1,
                    command_type: COMMAND_TYPE_HIGHSCORES_QUERY_V1,
                    payload,
                };
                let loyalty = payload(LOYALTY_CATEGORY)?;
                let experience = payload(1)?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(&offered, &[9]).ok_or("select 9")?;
                assert!(matches!(
                    route(&Owner, &admitted, command(&loyalty)).await,
                    Dispatch::Highscores(HighscoresReply::Terminal(
                        FoundationProtocolError::NotSupported
                    ))
                ));
                assert!(accepted(
                    route(&Owner, &admitted, command(&experience)).await,
                    1
                ));
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(&offered, &[9, CAPABILITY_HIGHSCORES_LOYALTY_V1])
                        .ok_or("select 498")?;
                assert!(accepted(
                    route(&Owner, &admitted, command(&loyalty)).await,
                    LOYALTY_CATEGORY
                ));
                Ok(())
            })
    }
}
