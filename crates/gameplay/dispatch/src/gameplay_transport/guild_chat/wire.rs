//! Negotiation and transport adapters; the connection slots delegate here.
use super::super::connection::FreshAdmissionAuthority;
use super::*;
use oteryn_protocol_oteryn::chat::guild_room::{self, GuildChatSnapshot};
use oteryn_protocol_oteryn::chat::{self, ChatIntent, ChatRoomSet};

pub(in super::super) async fn snapshot<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    rooms: ChatRoomSet,
) -> Option<(u32, Vec<u8>)> {
    if admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_CHAT_GUILD_ROOM_V1)
    {
        let guild = authority.chat_guild_admit(admitted).await;
        Some((
            guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1,
            guild_room::encode_snapshot(&GuildChatSnapshot { rooms, guild }).ok()?,
        ))
    } else {
        Some((chat::SNAPSHOT_TYPE_CHAT_V1, chat::encode_chat_rooms(rooms)))
    }
}

pub(in super::super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    command_id: u64,
    payload: &[u8],
) -> Result<
    Option<(
        ChatIntentResult,
        Option<super::super::actor_spell::SpellCastOutcome>,
    )>,
    (),
> {
    let intent = chat::decode_chat_intent(payload).map_err(|_| ())?;
    match intent {
        ChatIntent::Guild { text } => {
            if !admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CHAT_GUILD_ROOM_V1)
            {
                return Err(());
            }
            Ok(Some((
                authority.chat_guild_intent(admitted, text).await,
                None,
            )))
        }
        intent => Ok(authority
            .chat_intent_completion(actor, admitted.game_session_id, command_id, intent)
            .await),
    }
}
