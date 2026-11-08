use super::*;

impl Pushed {
    /// Folds `newer` over `self`: its position and entries win.
    pub(super) fn absorb(&mut self, newer: Self) {
        self.scope_changed |= newer.scope_changed;
        super::super::feature_0023_presentation::protocol::absorb_visuals(
            &mut self.visuals,
            newer.visuals,
        );
        super::super::feature_0023_presentation::protocol::absorb_sounds(
            &mut self.sounds,
            newer.sounds,
        );
        for text in newer.notices.tutorial {
            self.notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::Tutorial(
                    text,
                ));
        }
        self.notices.save_minutes = newer.notices.save_minutes.or(self.notices.save_minutes);
        self.served_channel = newer.served_channel.or(self.served_channel);
        self.world_clock = newer.world_clock.or(self.world_clock);
        self.world_weather = newer.world_weather.or(self.world_weather);
        self.map = newer.map.or(self.map.take());
        self.own = newer.own.or(self.own);
        self.entities = newer.entities.or(self.entities.take());
        self.appearances = newer.appearances.or(self.appearances.take());
        self.server_messages.append(newer.server_messages);
        self.achievements.extend(newer.achievements);
        if newer.chat_reset {
            self.chat_lines.clear();
            self.chat_reset = true;
        }
        self.chat_lines.extend(newer.chat_lines);
        self.chat_rooms = newer.chat_rooms.or(self.chat_rooms);
        self.vip_presence = newer.vip_presence.or(self.vip_presence.take());
        if newer.markers_reset {
            self.markers.clear();
            self.markers_reset = true;
        }
        self.quests.extend(newer.quests);
        for entry in newer.markers {
            self.set_marker(entry);
        }
    }
}
