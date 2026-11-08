//! Non-blocking projection and chronological input dispatch for the play shell.
use super::{
    Input, PlayEnd, PlayEvent, PlayLink, PlayView, feature_0011_world_interaction,
    feature_local_cursor_spells, feature_local_spell_action_bar,
};
use oteryn_platform_client::native_login::PublicClass;

impl PlayView {
    /// One shell tick, never waiting on the network: applies every outcome the session task has
    /// delivered, then hands it the next step if one is due. A session end returns to login with
    /// its public class, a committed logout to character selection; a view error is a render
    /// failure.
    pub fn tick(&mut self, link: &PlayLink) -> Result<(), PlayEnd> {
        #[cfg(windows)]
        link.player_trade_present();
        const UNAVAILABLE: PlayEnd = PlayEnd::Ended(PublicClass::SessionUnavailable);
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::ActionFeedback(line) => self.action_line = line,
                PlayEvent::Stepped(outcome) => {
                    self.apply(&outcome).map_err(|_error| UNAVAILABLE)?;
                }
                PlayEvent::Pushed(pushed) => {
                    self.apply_pushed(&pushed).map_err(|_error| UNAVAILABLE)?;
                    self.enqueue(pushed.sounds);
                }
                PlayEvent::Combat(report) => self.apply_combat(report),
                PlayEvent::LogoutRefused(result) => self.apply_logout_refusal(result),
                PlayEvent::LoggedOut => return Err(PlayEnd::LoggedOut),
                PlayEvent::Ended(class) => return Err(PlayEnd::Ended(class)),
            }
        }
        self.apply_target_turn(link).map_err(|_error| UNAVAILABLE)?;
        self.advance_presentation(std::time::Instant::now())
            .map_err(|_error| UNAVAILABLE)?;
        self.world_context_feedback = link.world_context_feedback();
        self.project_death(link).map_err(|_error| UNAVAILABLE)?;
        self.vitals = link.actor_vitals();
        let party = link.party_skull_panel();
        let icon_now = std::time::Instant::now();
        if self.preferences().creature_bars
            && party.selected
            && !party.pending
            && self.party_icons_refresh.is_none_or(|last| {
                icon_now.duration_since(last) >= std::time::Duration::from_secs(1)
            })
        {
            link.party_skull_request(oteryn_client_ui::party_skull::Intent::Query);
            self.party_icons_refresh = Some(icon_now);
        }
        self.party_icons = party.icons;
        self.project_presentation(std::time::Instant::now())
            .map_err(|_| UNAVAILABLE)?;
        let battle = link.battle_instinct_status_panel();
        self.battle_instinct = if battle.selected && battle.error.is_none() {
            battle.snapshot
        } else {
            None
        };
        let now = std::time::Instant::now();
        if battle.selected
            && self
                .battle_instinct_refresh
                .is_none_or(|last| now.duration_since(last) >= std::time::Duration::from_secs(2))
        {
            link.battle_instinct_status_request();
            self.battle_instinct_refresh = Some(now);
        }

        self.target_walk_feedback = link.target_walk_feedback();
        // One chronological queue: each input goes out in the order it was given. A step still
        // in flight holds back the arrow and what was given after it.
        while let Some(&input) = self.inputs.front() {
            match input {
                Input::CastAt(spell, position) => {
                    feature_local_cursor_spells::input::send_cast_at(link, spell, position);
                }
                Input::Cast(spell) => feature_local_spell_action_bar::input::send_cast(link, spell),
                Input::Attack(target) => {
                    feature_0011_world_interaction::input::send_attack(link, target)
                }
                Input::TargetWalk(position) => {
                    if !link.request_target_walk(position) {
                        break;
                    }
                }
                Input::Logout => link.request_logout(),
                Input::Arrow(_) => match self.send_step().map_err(|_error| UNAVAILABLE)? {
                    Some(direction) => link.request(direction),
                    None => break,
                },
            }
            if !matches!(input, Input::Arrow(_)) {
                self.inputs.pop_front();
            }
        }
        if self.inputs.is_empty()
            && let Some(direction) = self.send_step().map_err(|_error| UNAVAILABLE)?
        {
            link.request(direction);
        }
        Ok(())
    }
}
