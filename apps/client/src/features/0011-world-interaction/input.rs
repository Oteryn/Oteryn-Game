use crate::input::Targetable;
use crate::play::{Input, PlayView};
use oteryn_renderer::{BatchError, TileCoord};

impl PlayView {
    /// A click on `tile`: a drawn object or actor is selected, any other tile becomes the walk
    /// goal.
    pub fn click(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        if self.death.dead || crate::play::feature_0450_server_modal::view::active() {
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
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn creature_context(
        &self,
        target: oteryn_session::EntityRef,
        link: &PlayLink,
    ) -> Option<Menu> {
        self.entities.iter().find(|row| {
            row.entity == target
                && row.position.floor == self.floor
                && matches!(row.detail, oteryn_session::EntityDetail::Actor { .. })
        })?;
        let mailbox = lock(&link.mailbox);
        if mailbox.world_context.pending {
            return None;
        }
        let menu = Menu::new(
            oteryn_client_runtime::world_interaction::Target::Creature(target),
            mailbox.world_context.look,
            false,
        );
        (!menu.entries.is_empty()).then_some(menu)
    }
    pub fn world_context(&self, tile: TileCoord, link: &PlayLink) -> Option<Menu> {
        self.world_context_for(tile, link, false)
    }
    fn world_context_for(
        &self,
        tile: TileCoord,
        link: &PlayLink,
        prefer_item: bool,
    ) -> Option<Menu> {
        let target = self.world_context_target_for(tile, prefer_item)?;
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
        if let Some(crate::play::GroundItem::Map { header, .. }) = if prefer_item {
            self.scene.item_target_at(tile)
        } else {
            self.scene.target_at(tile).and_then(|target| target.item)
        } {
            menu.binding = Some((
                header.content_generation,
                header.bundle_digest,
                header.reset_epoch,
            ));
        }
        (!menu.entries.is_empty()).then_some(menu)
    }
    pub fn world_context_choose(&self, link: &PlayLink, menu: &Menu, action: Action) -> bool {
        let prefer_item = matches!(action, Action::Use | Action::UseWith | Action::UseRune);
        if !menu.entries.iter().any(|(_, entry)| *entry == action)
            || menu.tile.is_some_and(|tile| {
                self.world_context_target_for(tile, prefer_item).as_ref() != Some(&menu.target)
            })
        {
            lock(&link.mailbox)
                .world_context
                .set_feedback(tr!("Target changed").into(), std::time::Instant::now());
            return false;
        }
        if let Some(tile) = menu.tile {
            let Some(current) = self.world_context_for(tile, link, prefer_item) else {
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
        if menu.tile.is_none() {
            use oteryn_client_runtime::world_interaction::Target;
            let carried = match menu.target {
                Target::Item(handle) => {
                    mailbox
                        .inventory
                        .inventory
                        .as_ref()
                        .is_some_and(|inventory| {
                            inventory
                                .main_backpack
                                .iter()
                                .chain(&inventory.entries)
                                .chain(inventory.equipment.iter().map(|equipped| &equipped.item))
                                .any(|item| item.handle == handle)
                        })
                        || mailbox.containers.views.as_ref().is_some_and(|containers| {
                            containers.views.iter().any(|view| {
                                view.container_handle == handle
                                    || view.entries.iter().any(|item| item.handle == handle)
                            })
                        })
                }
                Target::Creature(entity) => self.entities.iter().any(|row| {
                    row.entity == entity
                        && row.position.floor == self.floor
                        && matches!(row.detail, oteryn_session::EntityDetail::Actor { .. })
                }),
                _ => false,
            };
            if !carried {
                mailbox
                    .world_context
                    .set_feedback(tr!("Target changed").into(), std::time::Instant::now());
                return false;
            }
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

impl PlayView {
    /// Ordinary modifier clicks use the same observed target and negotiated command as menus.
    pub fn modifier_click(
        &mut self,
        tile: TileCoord,
        link: &PlayLink,
        modifier: oteryn_input_actions::Modifier,
    ) -> bool {
        use oteryn_input_actions::Modifier;
        if self.gameplay_input_blocked(link) {
            return false;
        }
        if modifier == Modifier::Alt {
            return self.attack_toggle_click(tile, link);
        }
        let Some(menu) = self.world_context_for(tile, link, modifier == Modifier::Control) else {
            return false;
        };
        let action = match modifier {
            Modifier::Shift => Action::Look,
            Modifier::Control => {
                let multi_use = match self.scene.item_target_at(tile) {
                    Some(crate::play::GroundItem::Map { item, .. }) => {
                        self.scene.item_multi_use(item.appearance_id)
                    }
                    _ => false,
                };
                if multi_use {
                    Action::UseWith
                } else {
                    Action::Use
                }
            }
            _ => return false,
        };
        self.world_context_choose(link, &menu, action)
    }
}
