use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::battle::{self, Action};
use oteryn_renderer::TileCoord;
impl crate::play::PlayView {
    /// Alt+click on an opponent: stops the attack when it is the current target, else attacks it.
    pub fn attack_toggle_click(&self, tile: TileCoord, link: &PlayLink) -> bool {
        // Same guard as `PlayView::click`: no world input while dead or under a server modal.
        if link.lens_active()
            || self.death.dead
            || lock(&link.mailbox).world_input_blocked()
            || crate::play::feature_0450_server_modal::view::active()
        {
            return false;
        }
        let own = self.scene.own_identity();
        let Some(target) = self
            .scene
            .target_at(tile)
            .and_then(|target| target.entity)
            .filter(|target| {
                Some(target.identity) != own
                    && self.entities.iter().any(|entity| {
                        entity.entity == *target
                            && matches!(
                                entity.kind,
                                oteryn_session::EntityKind::Player
                                    | oteryn_session::EntityKind::Creature
                            )
                    })
            })
        else {
            return false;
        };
        let attacking = lock(&link.mailbox)
            .battle
            .combat
            .is_some_and(|state| state.target == Some(target));
        link.request_combat(if attacking {
            crate::combat_input::CombatCommand::Stop
        } else {
            crate::combat_input::CombatCommand::Attack(target)
        });
        true
    }
}
impl PlayLink {
    pub fn battle_request(&self, action: Action) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = battle::request(&mut mailbox.battle, action)
            && self.commands.send(PlayCommand::Battle(command)).is_err()
        {
            mailbox.battle.pending = false;
            mailbox.battle.pending_command = None;
        }
    }
    pub fn battle_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !battle::toggle_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.battle.shutdown {
                return true;
            }
            mailbox.battle.open = !mailbox.battle.open;
            if !mailbox.battle.open {
                return true;
            }
        }
        #[cfg(windows)]
        super::view::open_window(self);
        true
    }
}
