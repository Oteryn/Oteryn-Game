use crate::combat_input::{CombatCommand, PlayCommand};
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::battle::{self, Action};
use oteryn_renderer::TileCoord;
impl crate::play::PlayView {
    /// Shared widgets obey the same gameplay gate as world target selection.
    #[must_use]
    pub fn gameplay_input_blocked(&self, link: &PlayLink) -> bool {
        link.lens_active()
            || self.death.dead
            || lock(&link.mailbox).world_input_blocked()
            || crate::play::feature_0450_server_modal::view::active()
    }

    /// The attackable player or monster on `tile` (never the own character or an NPC), and
    /// whether it is the current attack target.
    #[must_use]
    pub fn attack_context(&self, tile: TileCoord, link: &PlayLink) -> Option<bool> {
        self.attack_target(tile, link)
            .map(|(_, attacking)| attacking)
    }
    pub(crate) fn attack_target(
        &self,
        tile: TileCoord,
        link: &PlayLink,
    ) -> Option<(oteryn_session::EntityRef, bool)> {
        let own = self.scene.own_identity();
        let target = self
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
            })?;
        let attacking = lock(&link.mailbox)
            .battle
            .combat
            .is_some_and(|state| state.target == Some(target));
        Some((target, attacking))
    }

    /// Alt+click on an opponent: stops the attack when it is the current target, else attacks it.
    pub fn attack_toggle_click(&self, tile: TileCoord, link: &PlayLink) -> bool {
        // Same guard as `PlayView::click`: no world input while dead or under a server modal.
        if self.gameplay_input_blocked(link) {
            return false;
        }
        let Some((target, attacking)) = self.attack_target(tile, link) else {
            return false;
        };
        link.request_context_combat(target, attacking)
    }
}
impl PlayLink {
    pub(crate) fn request_context_combat(
        &self,
        target: oteryn_session::EntityRef,
        attacking: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked()
            || mailbox
                .battle
                .combat
                .is_some_and(|state| state.target == Some(target))
                != attacking
        {
            return false;
        }
        let command = if attacking {
            CombatCommand::Stop
        } else {
            CombatCommand::Attack(target)
        };
        mailbox.cancel_walk_for_combat(command);
        self.commands.send(PlayCommand::Combat(command)).is_ok()
    }

    pub fn battle_request(&self, action: Action) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.properties.pvp_expert.pending {
            return;
        }
        if let Some(command) = battle::request(&mut mailbox.battle, action) {
            mailbox.cancel_walk_for_combat(command);
            if self.commands.send(PlayCommand::Battle(command)).is_err() {
                mailbox.battle.pending = false;
                mailbox.battle.pending_command = None;
            }
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
        true
    }
}

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn cancel_walk_for_combat(&mut self, command: CombatCommand) {
        if matches!(command, CombatCommand::Attack(_) | CombatCommand::Stop) {
            oteryn_client_ui::target_move::cancel(&mut self.target_walk);
        }
    }
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    #[test]
    fn pending_secure_mode_serializes_battle_and_expert_commands() {
        let (link, mut commands, _) = crate::play::play_channel();
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.properties.pvp_expert.selected = true;
            mailbox.properties.pvp_expert.fields = ["1", "1", "0", "0"].map(str::to_owned);
            mailbox.battle.combat = Some(oteryn_session::ActorCombatState {
                target: None,
                modes: oteryn_session::FightModes::DEFAULT,
                in_fight: false,
            });
        }
        link.pvp_expert_request(oteryn_client_ui::pvp_expert::Intent::Apply);
        assert!(matches!(commands.try_recv(), Ok(PlayCommand::Feature(_))));
        link.battle_request(Action::Fight(
            oteryn_client_runtime::attack::FightMode::Defensive,
        ));
        assert!(commands.try_recv().is_err());
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.properties.pvp_expert.pending = false;
        }
        link.battle_request(Action::Chase(
            oteryn_client_runtime::attack::ChaseMode::Chase,
        ));
        assert!(matches!(commands.try_recv(), Ok(PlayCommand::Battle(_))));
        link.pvp_expert_request(oteryn_client_ui::pvp_expert::Intent::Apply);
        assert!(commands.try_recv().is_err());
    }
}

#[cfg(test)]
#[test]
fn target_change_between_context_check_and_submission_refuses_stop() {
    use oteryn_session::{ActorCombatState, EntityRef, FightModes};
    let (link, mut commands, _) = crate::play::play_channel();
    let first = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let second = EntityRef {
        identity: [2; 16],
        generation: 1,
    };
    let combat = |target| {
        Some(ActorCombatState {
            target: Some(target),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
    };
    lock(&link.mailbox).battle.combat = combat(first);
    let was_attacking = lock(&link.mailbox)
        .battle
        .combat
        .is_some_and(|state| state.target == Some(first));
    assert!(was_attacking);
    lock(&link.mailbox).battle.combat = combat(second);
    assert!(!link.request_context_combat(first, was_attacking));
    assert!(commands.try_recv().is_err());
    assert!(link.request_context_combat(second, true));
    assert!(matches!(
        commands.try_recv(),
        Ok(PlayCommand::Combat(CombatCommand::Stop))
    ));
}
