use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_session::{EntityKind, EntityRef};
impl PlayView {
    pub fn selected_npc(&self) -> Option<EntityRef> {
        self.npc_at(self.scene.target()?.tile)
    }
    pub(in crate::play) fn npc_at(&self, tile: oteryn_renderer::TileCoord) -> Option<EntityRef> {
        self.entities
            .iter()
            .rev()
            .find(|e| {
                matches!(
                    e.kind,
                    EntityKind::Npc | EntityKind::Creature | EntityKind::Player
                ) && e.position.floor == self.floor
                    && e.position.x == tile.x
                    && e.position.y == tile.y
            })
            .filter(|e| e.kind == EntityKind::Npc)
            .map(|e| e.entity)
    }
}
impl PlayLink {
    pub fn talk_to_npc(&self, npc: EntityRef, text: &str) {
        self.npc_dialogue_submit(npc, text);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
    }
    /// Returns whether the retained NPC owner enqueued this exact talk request.
    pub fn npc_dialogue_submit(&self, npc: EntityRef, text: &str) -> bool {
        self.npc_dialogue_enqueue(npc, text, None)
    }
    /// A displayed reply must still address the same actor and server conversation atomically.
    pub fn npc_dialogue_reply(&self, npc: EntityRef, lines: &[String], text: &str) -> bool {
        self.npc_dialogue_enqueue(npc, text, Some(lines))
    }
    fn npc_dialogue_enqueue(&self, npc: EntityRef, text: &str, context: Option<&[String]>) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if context.is_some_and(|lines| {
            !mailbox.dialogue.open
                || mailbox.dialogue.target != Some(npc)
                || mailbox.dialogue.lines.as_slice() != lines
        }) {
            return false;
        }
        let Some(intent) =
            oteryn_client_ui::npc_dialogue::request(&mut mailbox.dialogue, npc, text)
        else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::NpcTalk(intent))
            .is_err()
        {
            mailbox.dialogue = Default::default();
            return false;
        }
        true
    }
    pub fn npc_dialogue_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        view: &PlayView,
    ) -> bool {
        if !oteryn_client_ui::npc_dialogue::open_key(events) {
            return false;
        }
        if let Some(npc) = view.selected_npc() {
            self.talk_to_npc(npc, "hi");
        }
        true
    }
}

#[cfg(any(windows, test))]
impl PlayLink {
    pub(in crate::play) fn npc_context_target(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        view: &PlayView,
    ) -> Option<EntityRef> {
        use oteryn_input_actions::{ButtonState, MouseButton, NormalizedInputEvent as Event};
        if !lock(&self.mailbox).dialogue.selected {
            return None;
        }
        let mut target = None;
        for event in events {
            match event {
                Event::PointerMoved { position, .. } => {
                    lock(&self.mailbox).dialogue_pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())));
                }
                Event::FocusChanged { focused: false } | Event::DeviceLost => {
                    lock(&self.mailbox).dialogue_pointer = None;
                    target = None;
                }
                _ => {}
            }
            if matches!(
                event,
                Event::MouseButton {
                    button: MouseButton::SECONDARY,
                    state: ButtonState::Pressed,
                    ..
                }
            ) {
                let pointer = lock(&self.mailbox).dialogue_pointer;
                if let Some((x, y)) = pointer
                    && let Some(tile) = crate::input::click_tile(view.scene().view(), x, y)
                    && let Some(npc) = view.npc_at(tile)
                {
                    target = Some(npc);
                }
            }
        }
        target
    }
}

#[cfg(windows)]
impl PlayLink {
    pub fn npc_context_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        view: &PlayView,
    ) -> bool {
        let Some(npc) = self.npc_context_target(events, view) else {
            return false;
        };
        super::view::context_menu(self, npc);
        true
    }
}
