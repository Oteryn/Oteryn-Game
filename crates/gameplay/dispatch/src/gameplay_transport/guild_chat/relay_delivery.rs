use super::*;

impl ChatRuntime {
    pub(super) fn deliver_relay(&self, line: &RelayLine) {
        // Presence is an invalidation hint, never a line and never authoritative status.
        if let RelayDestination::Presence { target, online } = line.destination {
            self.vip.hint(target, online);
            return;
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let name = line.sender_name.as_str();
        match line.destination {
            RelayDestination::Presence { .. } => {}
            RelayDestination::PrivateChannel { channel_id, epoch } => {
                for (session, entry) in sessions.iter_mut() {
                    if self.private_world.accepts_relay(
                        *session,
                        channel_id,
                        epoch,
                        line.sent_at_micros,
                    ) {
                        entry.egress.push_private_channel(
                            oteryn_protocol_oteryn::chat::private_channel::PrivateChannelLine {
                                channel_id,
                                speaker_name: name.to_owned(),
                                text: line.text.as_str().to_owned(),
                            },
                            crate::chat::egress::PrivateChannelChatDestination {
                                channel_id,
                                epoch,
                                sent_at_micros: line.sent_at_micros,
                            },
                        );
                    }
                }
            }
            RelayDestination::Party { party_id } => {
                for (session, entry) in sessions.iter_mut() {
                    if !self.party.negotiated(*session) {
                        continue;
                    }
                    entry.egress.push_party(
                        name.to_owned(),
                        line.text.as_str().to_owned(),
                        crate::chat::egress::PartyChatDestination {
                            party_id,
                            sent_at_micros: line.sent_at_micros,
                        },
                    );
                }
            }
            RelayDestination::Guild { guild_id, rank } => {
                for (session, entry) in sessions.iter_mut() {
                    if self.guild.negotiated(*session) {
                        entry.egress.push(crate::chat::guild_room::guild_line(
                            guild_id, rank, name, &line.text,
                        ));
                    }
                }
            }
            RelayDestination::Private { recipient } => {
                for entry in sessions.values_mut() {
                    if entry.character.as_ref() == Some(recipient.as_bytes()) {
                        entry.egress.push(private_line(name, &line.text));
                    }
                }
            }
            RelayDestination::Room { room } => {
                for entry in sessions.values_mut() {
                    if entry.rooms.contains(room) && entry.public_rooms.can_join(room) {
                        entry.egress.push(room_line(room, name, &line.text));
                    }
                }
            }
        }
    }
}

impl ChatRuntime {
    pub(in super::super) fn private_channel_destination(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> Option<crate::chat::egress::PrivateChannelChatDestination> {
        self.sessions
            .lock()
            .ok()?
            .get(&session)?
            .egress
            .private_channel_destination()
    }

    pub(in super::super) fn accept_private_channel_line(
        &self,
        admitted: &crate::gameplay_transport::connection::AdmittedSession,
        candidate: &ChatLine,
        destination: crate::chat::egress::PrivateChannelChatDestination,
        allowed: bool,
    ) -> Option<ChatLine> {
        let mut sessions = self.sessions.lock().ok()?;
        if !self.private_world.matches(admitted) {
            return None;
        }
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if entry.egress.peek() != Some(candidate)
            || entry.egress.private_channel_destination() != Some(destination)
        {
            return None;
        }
        if allowed {
            entry.egress.next_for_write()
        } else {
            entry.egress.next_for_write();
            entry.egress.delivered();
            None
        }
    }
}
