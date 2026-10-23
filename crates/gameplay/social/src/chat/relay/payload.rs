use super::*;

impl RelayLine {
    pub(super) fn encode(&self) -> Result<Vec<u8>, RelayError> {
        if self.text.as_str().len() > TEXT_MAX_BYTES {
            return Err(RelayError::Oversized);
        }
        let mut out = Vec::with_capacity(PLAINTEXT_MAX_BYTES);
        match self.destination {
            RelayDestination::Private { .. } => out.push(KIND_PRIVATE),
            RelayDestination::Room { .. } => out.push(KIND_ROOM),
            RelayDestination::Guild { .. } => out.push(super::super::guild_room::KIND_GUILD),
        }
        out.extend_from_slice(&self.message_id);
        out.extend_from_slice(&self.sent_at_micros.to_be_bytes());
        match self.destination {
            RelayDestination::Private { recipient } => out.extend_from_slice(recipient.as_bytes()),
            RelayDestination::Room { room } => out.push(room as u8),
            RelayDestination::Guild { guild_id, rank } => {
                super::super::guild_room::encode_destination(&mut out, guild_id, rank)
            }
        }
        push_prefixed(&mut out, self.sender_name.as_str().as_bytes())?;
        push_prefixed(&mut out, self.text.as_str().as_bytes())?;
        Ok(out)
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, RelayError> {
        let mut cursor = Cursor(bytes);
        let kind = cursor.take(1)?[0];
        let message_id = cursor.array()?;
        let sent_at_micros = u64::from_be_bytes(cursor.array()?);
        let destination = match kind {
            // A zero or non-UUIDv7 recipient is refused like any other malformed identifier.
            KIND_PRIVATE => RelayDestination::Private {
                recipient: CharacterId::decode(&cursor.array::<ID_BYTES>()?)
                    .map_err(|_| RelayError::BadPayload)?,
            },
            KIND_ROOM => {
                let code = cursor.take(1)?[0];
                let room = ChatRoom::ALL
                    .into_iter()
                    .find(|room| *room as u8 == code)
                    .ok_or(RelayError::BadPayload)?;
                RelayDestination::Room { room }
            }
            super::super::guild_room::KIND_GUILD => {
                super::super::guild_room::decode_destination(&mut cursor)?
            }
            _ => return Err(RelayError::BadPayload),
        };
        // Only a name the naming policy admits is accepted, as on the sending side.
        let sender_name = CharacterName::parse(&cursor.prefixed(SENDER_NAME_MAX_BYTES)?)
            .map_err(|_| RelayError::BadPayload)?;
        let raw = cursor.prefixed(TEXT_MAX_BYTES)?;
        // Only the canonical (already trimmed) form a sender can seal is accepted.
        let text = ChatText::parse(&raw)
            .ok()
            .filter(|text| text.as_str() == raw)
            .ok_or(RelayError::BadPayload)?;
        if !cursor.0.is_empty() {
            return Err(RelayError::BadPayload);
        }
        Ok(Self {
            message_id,
            sent_at_micros,
            destination,
            sender_name,
            text,
        })
    }
}
