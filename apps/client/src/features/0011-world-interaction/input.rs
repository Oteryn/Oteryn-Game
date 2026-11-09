use crate::input::Targetable;
use crate::play::{Input, PlayView};
use oteryn_renderer::{BatchError, TileCoord};

impl PlayView {
    /// A click on `tile`: a drawn object or actor is selected, any other tile becomes the walk
    /// goal.
    pub fn click(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        if self.death.dead {
            return Ok(());
        }
        if self.cast_at_cursor(tile) {
            return Ok(());
        }
        match self.scene.select_tile(tile)? {
            None => self.walk_to(tile),
            Some(Targetable {
                entity: Some(target),
                ..
            }) => {
                if self.entities.iter().any(|entity| {
                    entity.entity == target && entity.kind == oteryn_session::EntityKind::Creature
                }) {
                    self.queue(Input::Attack(target));
                }
            }
            Some(_) => {}
        }
        Ok(())
    }
}

pub(in crate::play) fn send_attack(
    link: &crate::play::PlayLink,
    target: oteryn_session::EntityRef,
) {
    link.request_combat(crate::combat_input::CombatCommand::Attack(target));
}

use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::world_interaction::{Action, Command};
use oteryn_client_ui::world_interaction::menu::Menu;
impl PlayView {
    pub fn world_context(&self, tile: TileCoord, link: &PlayLink) -> Option<Menu> {
        let target = self.world_context_target(tile)?;
        let mailbox = lock(&link.mailbox);
        if mailbox.world_context.pending {
            return None;
        }
        let mut menu = Menu::new(
            target,
            mailbox.world_context.look,
            mailbox.world_context.bed,
        )
        .with_targeting(mailbox.world_context.use_with);
        menu.tile = Some(tile);
        if let Some(crate::play::GroundItem::Map { header, .. }) =
            self.scene.target_at(tile).and_then(|target| target.item)
        {
            menu.binding = Some((
                header.content_generation,
                header.bundle_digest,
                header.reset_epoch,
            ));
        }
        (!menu.entries.is_empty()).then_some(menu)
    }
    pub fn world_context_choose(&self, link: &PlayLink, menu: &Menu, action: Action) -> bool {
        if !menu.entries.iter().any(|(_, entry)| *entry == action)
            || menu
                .tile
                .and_then(|tile| self.world_context_target(tile))
                .as_ref()
                != Some(&menu.target)
        {
            lock(&link.mailbox)
                .world_context
                .set_feedback(tr!("Target changed").into(), std::time::Instant::now());
            return false;
        }
        if let Some(tile) = menu.tile {
            let Some(current) = self.world_context(tile, link) else {
                return false;
            };
            if current.binding != menu.binding {
                lock(&link.mailbox)
                    .world_context
                    .set_feedback(tr!("Target changed").into(), std::time::Instant::now());
                return false;
            }
        }
        let mut mailbox = lock(&link.mailbox);
        if mailbox.world_context.pending {
            return false;
        }
        let current = Menu::new(
            menu.target.clone(),
            mailbox.world_context.look,
            mailbox.world_context.bed,
        )
        .with_targeting(mailbox.world_context.use_with);
        if !current.entries.iter().any(|(_, entry)| *entry == action) {
            return false;
        }
        if matches!(action, Action::UseWith | Action::UseRune) {
            let oteryn_client_runtime::world_interaction::Target::Item(source) = menu.target else {
                return false;
            };
            mailbox.world_context.begin_targeting(source);
            mailbox.world_context.rune_targeting = action == Action::UseRune;
            return true;
        }
        if link
            .commands
            .send(crate::combat_input::PlayCommand::WorldContext(Command {
                target: menu.target.clone(),
                action,
                selection: menu.tile.map(|tile| {
                    oteryn_client_runtime::world_interaction::Selection {
                        position: oteryn_session::ActorPosition {
                            x: tile.x,
                            y: tile.y,
                            floor: self.floor,
                        },
                        binding: menu.binding,
                    }
                }),
            }))
            .is_err()
        {
            return false;
        }
        mailbox.world_context.pending = true;
        mailbox
            .world_context
            .set_feedback(tr!("Waiting for server").into(), std::time::Instant::now());
        true
    }
}
impl PlayLink {
    pub fn world_context_feedback(&self) -> Option<String> {
        lock(&self.mailbox)
            .world_context
            .feedback(std::time::Instant::now())
    }
}
