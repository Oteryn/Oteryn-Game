use super::*;

impl ChatRuntime {
    pub(super) fn deliver_relay(&self, line: &RelayLine) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let name = line.sender_name.as_str();
        match line.destination {
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
                    if entry.rooms.contains(room) {
                        entry.egress.push(room_line(room, name, &line.text));
                    }
                }
            }
        }
    }
}
