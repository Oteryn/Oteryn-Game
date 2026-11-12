//! Targeting consumes primary clicks before walking or attacking.
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};
use oteryn_input_actions::{ButtonState, NormalizedInputEvent};
use oteryn_renderer::TileCoord;

impl PlayLink {
    pub fn inventory_use_rune(&self) -> bool {
        self.inventory_use(true)
    }
    pub fn inventory_use_with(&self) -> bool {
        self.inventory_use(false)
    }
    fn inventory_use(&self, rune: bool) -> bool {
        Self::inventory_use_shared(&self.mailbox, &self.commands, rune)
    }
    pub(in crate::play) fn inventory_use_shared(
        shared: &crate::play::mailbox::SharedMailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
        rune: bool,
    ) -> bool {
        let mut mailbox = lock(shared);
        if mailbox.inventory_shutdown || mailbox.world_context.pending {
            return false;
        }
        if !mailbox.world_context.use_with {
            mailbox.world_context.set_feedback(
                tr!("Use with unavailable").into(),
                std::time::Instant::now(),
            );
            return false;
        }
        let Some(source) = mailbox.inventory.selected_item().map(|item| item.handle) else {
            mailbox.world_context.set_feedback(
                tr!("Use with: select an inventory item").into(),
                std::time::Instant::now(),
            );
            return false;
        };
        if let Some(origin) = mailbox.world_context.targeting {
            if origin == source || rune || mailbox.world_context.rune_targeting {
                mailbox.world_context.set_feedback(
                    tr!("Use with: choose another target").into(),
                    std::time::Instant::now(),
                );
                return false;
            }
            mailbox.world_context.targeting = None;
            let command = Command {
                target: Target::Item(source),
                action: Action::UseOn(origin),
                selection: None,
            };
            if commands
                .send(crate::combat_input::PlayCommand::WorldContext(command))
                .is_err()
            {
                mailbox
                    .world_context
                    .set_feedback(tr!("Session unavailable").into(), std::time::Instant::now());
                return false;
            }
            mailbox.world_context.pending = true;
            mailbox.world_context.set_feedback(
                tr!("Use with: waiting for server").into(),
                std::time::Instant::now(),
            );
            return true;
        }
        mailbox.world_context.begin_targeting(source);
        mailbox.world_context.rune_targeting = rune;
        true
    }

    pub fn use_with_cancel_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !events.iter().any(|event| {
            matches!(event,
                NormalizedInputEvent::Key {
                    code, state: ButtonState::Pressed, repeat: false, ..
                } if *code == oteryn_input_actions::KeyCode::ESCAPE
            )
        }) {
            return false;
        }
        lock(&self.mailbox).world_context.cancel_targeting()
    }
}
impl PlayView {
    pub fn primary_click(
        &mut self,
        tile: TileCoord,
        link: &PlayLink,
    ) -> Result<(), oteryn_renderer::BatchError> {
        if !self.use_with_click(tile, link) {
            self.click(tile)?;
        }
        Ok(())
    }

    pub fn use_with_click(&self, tile: TileCoord, link: &PlayLink) -> bool {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.world_context.pending {
            mailbox.world_context.set_feedback(
                tr!("Use with: waiting for server").into(),
                std::time::Instant::now(),
            );
            return true;
        }
        let Some(source) = mailbox.world_context.targeting else {
            return false;
        };
        let position = oteryn_session::ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: self.floor,
        };
        let target = oteryn_client_ui::item_use::input::destination(
            mailbox.world_context.rune_targeting,
            self.world_context_target(tile),
            position,
        );
        if target == Target::Item(source) {
            mailbox.world_context.set_feedback(
                tr!("Use with: choose another target").into(),
                std::time::Instant::now(),
            );
            return true;
        }
        mailbox.world_context.targeting = None;
        let binding = match self.scene.target_at(tile).and_then(|target| target.item) {
            Some(crate::play::GroundItem::Map { header, .. }) => Some((
                header.content_generation,
                header.bundle_digest,
                header.reset_epoch,
            )),
            _ => None,
        };
        let command = Command {
            target,
            action: Action::UseOn(source),
            selection: Some(Selection { position, binding }),
        };
        if link
            .commands
            .send(crate::combat_input::PlayCommand::WorldContext(command))
            .is_ok()
        {
            mailbox.world_context.pending = true;
            mailbox.world_context.set_feedback(
                tr!("Use with: waiting for server").into(),
                std::time::Instant::now(),
            );
        } else {
            mailbox
                .world_context
                .set_feedback(tr!("Session unavailable").into(), std::time::Instant::now());
        }
        true
    }
}
