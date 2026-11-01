//! S04c public-channel negotiation. Existing CHAT_INTENT (13), room deltas and
//! private-channel list are extended only for peers selecting this capability.
pub const CAPABILITY_CHAT_PUBLIC_CHANNELS_V1: u32 = 259;

#[cfg(test)]
mod tests {
    use super::super::*;
    #[test]
    fn public_channel_room_ids_roundtrip_without_renumbering_existing_rooms()
    -> Result<(), ChatWireError> {
        assert_eq!(ChatRoom::Advertising as u8, 4);
        for room in ChatRoom::ALL {
            let intent = ChatIntent::OpenRoom(room);
            assert_eq!(decode_chat_intent(&encode_chat_intent(&intent)?)?, intent);
            let mut rooms = ChatRoomSet::default();
            rooms.insert(room);
            assert_eq!(decode_chat_rooms(&encode_chat_rooms(rooms))?, rooms);
        }
        assert!(decode_chat_rooms(&[8, 128, 2]).is_err());
        Ok(())
    }

    fn acceptance(selected: &[u32], resume: bool) -> Vec<u8> {
        let id = [1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1];
        let mut payload = Vec::new();
        crate::push_bytes(&mut payload, 1, &id);
        let capability_field = if resume {
            crate::push_scalar(&mut payload, 2, 1);
            crate::push_scalar(&mut payload, 4, 1);
            crate::push_scalar(&mut payload, 5, 1);
            6
        } else {
            crate::push_bytes(&mut payload, 2, &id);
            crate::push_bytes(&mut payload, 3, &id);
            crate::push_scalar(&mut payload, 4, 1);
            crate::push_scalar(&mut payload, 6, 1);
            crate::push_scalar(&mut payload, 7, u64::from(crate::PROTOCOL_MAJOR_V1));
            crate::push_scalar(
                &mut payload,
                8,
                u64::from(crate::TRANSPORT_PROFILE_TCP_TLS13_V1),
            );
            crate::push_scalar(&mut payload, 9, 1);
            10
        };
        for capability in selected {
            crate::push_scalar(&mut payload, capability_field, u64::from(*capability));
        }
        let mut frame = vec![8, if resume { 4 } else { 2 }];
        crate::push_bytes(&mut frame, 4, &payload);
        frame
    }

    #[test]
    fn public_channel_acceptance_and_resume_require_chat_and_private_channels()
    -> Result<(), crate::FoundationProtocolError> {
        let id = [1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1];
        for selected in [&[259][..], &[7, 259], &[80, 259], &[7, 80, 259]] {
            let accepted = crate::ServerAcceptedValue {
                game_session_id: crate::GameSessionId::decode(&id)?,
                world_id: crate::WorldId::decode(&id)?,
                channel_id: crate::ChannelId::decode(&id)?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: selected,
            };
            let resumed = crate::ServerResumeAcceptedValue {
                game_session_id: accepted.game_session_id,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: selected,
            };
            let complete = selected.contains(&7) && selected.contains(&80);
            if complete {
                crate::decode_wire_envelope(&crate::encode_server_accepted(&accepted)?)?;
                crate::decode_wire_envelope(&crate::encode_server_resume_accepted(&resumed)?)?;
            } else {
                assert_eq!(
                    crate::encode_server_accepted(&accepted),
                    Err(crate::FoundationProtocolError::InvalidCapabilitySet)
                );
                assert_eq!(
                    crate::encode_server_resume_accepted(&resumed),
                    Err(crate::FoundationProtocolError::InvalidCapabilitySet)
                );
            }
            for resume in [false, true] {
                let frame = acceptance(selected, resume);
                if complete {
                    crate::decode_wire_envelope(&frame)?;
                } else {
                    assert_eq!(
                        crate::decode_wire_envelope(&frame),
                        Err(crate::FoundationProtocolError::InvalidCapabilitySet)
                    );
                }
            }
        }
        Ok(())
    }
}
