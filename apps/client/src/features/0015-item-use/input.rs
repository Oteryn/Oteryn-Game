//! Targeting consumes primary clicks before walking or attacking.
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};
use oteryn_input_actions::{ButtonState, NormalizedInputEvent};
use oteryn_renderer::TileCoord;

impl PlayLink {
    /// Whether an item is waiting for a use-with target.
    pub fn use_with_armed(&self) -> bool {
        lock(&self.mailbox).world_context.targeting.is_some()
    }

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
            mailbox.use_with_pending = true;
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
        let use_with = lock(&self.mailbox).world_context.cancel_targeting();
        self.hireling_pick_cancel() || use_with
    }
}
impl PlayView {
    /// Tibia Global cancels an armed crosshair (Use with, a cursor spell or the banker's
    /// location pick) on a secondary click instead of opening the context menu or looting.
    pub fn secondary_click_cancels_targeting(&mut self, link: &PlayLink) -> bool {
        let use_with = lock(&link.mailbox).world_context.cancel_targeting();
        let spell = self.cursor_spell.take().is_some();
        link.hireling_pick_cancel() || use_with || spell
    }
    pub fn primary_click(
        &mut self,
        tile: TileCoord,
        link: &PlayLink,
    ) -> Result<(), oteryn_renderer::BatchError> {
        if !self.hireling_pick(tile, link) && !self.use_with_click(tile, link) {
            self.click(tile)?;
        }
        Ok(())
    }

    pub fn use_with_click(&self, tile: TileCoord, link: &PlayLink) -> bool {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.world_context.pending
            && (mailbox.use_with_pending || mailbox.world_context.targeting.is_some())
        {
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
        let selected_item = self.scene.item_target_at(tile);
        let realm_fire = matches!(selected_item, Some(crate::play::GroundItem::Map { item, position, .. })
            if item.appearance_id == 2124 && (position.x, position.y, position.floor) == (33080, 32583, 3)
                && matches!(item.origin, oteryn_session::world_map::MapOrigin::BaseOrdinal { ordinal: 1, .. }));
        let destroy_field = mailbox.world_context.rune_targeting && realm_fire && mailbox.inventory.rows().iter()
            .chain(mailbox.containers.views.iter().flat_map(|views| &views.views).flat_map(|view| &view.entries))
            .any(|item| item.handle == source && Some(item.item_definition_ref.get()) == env!("DESTROY_FIELD_DEFINITION").parse::<u32>().ok());
        let prefer_item = destroy_field || !mailbox.world_context.rune_targeting && self.scene.target().is_some_and(|entry| entry.tile == tile && entry.item.is_some());
        let target = oteryn_client_ui::item_use::input::destination(
            mailbox.world_context.rune_targeting && !destroy_field,
            self.world_context_target_for(tile, prefer_item),
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
        let binding_item = if prefer_item {
            selected_item
        } else {
            self.scene.target_at(tile).and_then(|target| target.item)
        };
        let binding = match binding_item {
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
            mailbox.use_with_pending = true;
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
