//! Existing product input owners, shared with headless caller tests.
#[path = "controls/container.rs"]
mod container;
use super::owner::HostOwner;
use crate::{input::MouseActions, spell::SpellHotkeys};
use oteryn_input_actions::{InputError, NormalizedInputEvent};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ContainerTool {
    BrowseField,
    Depot,
    Search,
    Sort,
}
impl ContainerTool {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::BrowseField => "Browse Field",
            Self::Depot => "Depot",
            Self::Search => "Depot Search",
            Self::Sort => "Container Sort",
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct ContainerToolRequest {
    pub generation: u64,
    pub tool: ContainerTool,
    pub position: Option<(i32, i32, i16)>,
    pub view: Option<(u8, oteryn_session::ItemHandle)>,
}
pub(super) struct Controls {
    pub(super) highscores_generation: Option<u64>,
    mouse: MouseActions,
    spells: SpellHotkeys,
    satellite: bool,
    container_tool: Option<ContainerToolRequest>,
    house: Option<u64>,
}
impl Controls {
    pub fn new() -> Result<Self, InputError> {
        Ok(Self {
            highscores_generation: None,
            mouse: MouseActions::new()?,
            spells: SpellHotkeys::new()?,
            satellite: false,
            container_tool: None,
            house: None,
        })
    }
    fn container(
        &mut self,
        link: &crate::play::PlayLink,
        events: &[NormalizedInputEvent],
        generation: u64,
    ) -> bool {
        let tool = if link.browse_field_input(events) {
            ContainerTool::BrowseField
        } else if link.depot_input(events) {
            ContainerTool::Depot
        } else if link.depot_search_input(events) {
            ContainerTool::Search
        } else if link.container_sort_input(events) {
            ContainerTool::Sort
        } else {
            return false;
        };
        // The Depot already has its own retained window (game_ui::depot) over `depot_panel().open`.
        if tool != ContainerTool::Depot {
            self.container_tool = Some(ContainerToolRequest {
                generation,
                tool,
                position: None,
                view: None,
            });
        }
        true
    }
    // Team Finder opens through the discovery panel and its retained Bevy window.
    fn discovery(link: &crate::play::PlayLink, events: &[NormalizedInputEvent]) -> bool {
        let handled = link.discovery_input(events);
        if handled && oteryn_client_ui::purchase_history::open_key(events) {
            link.store_set_open(true);
            link.purchase_history(false);
        }
        handled
    }
}
impl HostOwner {
    pub(super) fn route_input(&mut self, events: &[NormalizedInputEvent], blocked: bool) {
        let chat = self
            .play
            .as_ref()
            .is_some_and(|p| p.link.preference_chat_open());
        // Every reset reaches every stateful router, including releases suppressed by a modal.
        let reset = [NormalizedInputEvent::DeviceLost];
        let events = if blocked || self.modal_active() {
            &reset[..]
        } else {
            events
        };
        let controls = self.settings.control_events(events, chat);
        if let Some(play) = &mut self.play {
            play.view.observe_arrow_releases(&controls);
        }
        let (routed, open) = self.settings.input_in_context(&controls, chat);
        let clicks = self.controls.mouse.route(&routed);
        let spells = self.controls.spells.route(&routed);
        let custom = self.settings.take_custom_actions();
        if open {
            self.open_graphics();
            return;
        }
        if blocked || self.modal_active() {
            if let Some(play) = &mut self.play {
                let _ = play.view.world_pointer_input(&play.link, &reset);
                play.view.clear_bar_pointer(&play.link);
            }
            return;
        }
        if crate::bulletin::open_key(&routed) {
            self.login.bulletin.visible = true;
            return;
        }
        let generation = self.generation();
        let Some(play) = &mut self.play else {
            return;
        };
        if play.view.tutorial_input(&routed).unwrap_or(true)
            || play.view.notification_input(&routed)
            || play.link.use_with_cancel_input(&routed)
            || play.link.chat_input(&routed)
            || play.link.chat_public_input(&routed)
        {
            return;
        }
        play.view.observe_creature_pointer(&routed);
        play.view.observe_bar_pointer(&play.link);
        let mut hotkey_options = false;
        for action in custom {
            match &action {
                crate::play::CustomAction::Utility {
                    action: crate::play::UtilityAction::ShowOptionsHotkeys,
                } => hotkey_options = true,
                // Diagnostics never take a chord from the player's typing.
                crate::play::CustomAction::Utility {
                    action: crate::play::UtilityAction::ToggleDiagnosticsOverlay,
                } if chat => {}
                crate::play::CustomAction::Utility { action } => {
                    self.settings.run_utility(&play.link, *action);
                }
                _ => {
                    play.link.custom_hotkey_action(&play.view, &action);
                }
            }
        }
        if hotkey_options {
            self.show_hotkey_options();
            return;
        }
        if play.link.custom_hotkeys_input(&routed)
            || play.view.target_move_input(&play.link, &routed)
        {
            return;
        }
        if play.view.audio_input(&routed) {
            if let Err(error) = self.settings.capture(&mut play.view) {
                self.status = error.to_string();
            }
            return;
        }
        if super::input::satellite_key(&routed) {
            self.controls.satellite = true;
            return;
        }
        // Ctrl+Z stays native undo while an editor owns focus (the blocked path above).
        if routed.iter().any(|event| matches!(event,
            NormalizedInputEvent::Key { code, state: oteryn_input_actions::ButtonState::Pressed,
                modifiers, repeat: false }
            if code.get() == 29 && *modifiers == oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)))
        {
            play.link.bug_report_open();
            return;
        }
        if play.link.analyser_input(&routed)
            || play.link.mount_input(&routed)
            || play.link.appearance_input(&routed)
            || play.link.charm_progression_input(&routed)
            || play.link.cyclopedia_character_input(&routed)
            || play.link.item_catalogue_input(&routed)
            || play.link.daily_reward_input(&routed)
            || play.link.depot_input(&routed)
            || play.link.quest_input(&routed)
            || play.link.spell_catalog_input(&routed)
            || play.link.stash_input(&routed)
            || play.link.notices_input(&routed)
            || play.link.npc_dialogue_input(&routed, &play.view)
            || Controls::discovery(&play.link, &routed)
            || self.controls.container(&play.link, &routed, generation)
            || play.link.player_trade_input(&routed)
            || play.link.item_forge_input(&routed)
            || (routed.iter().any(|event| {
                matches!(
                    event,
                    NormalizedInputEvent::Key { code, .. }
                        if code.get() == 25 || *code == oteryn_input_actions::KeyCode::KEY_F
                )
            }) && play.link.vip_input(&routed))
            || play.link.wheel_input(&routed)
        {
            return;
        }
        // Windows the shared frontend presents with the generic form window
        // (game_ui::feature_forms), in the order the native shell routed them.
        if play.link.achievement_notices_input(&routed)
            || play.link.achievements_input(&routed)
            || play.link.cast_control_input(&routed)
            || play.link.cast_chat_input(&routed)
            || (oteryn_client_ui::boss_encounter::cooldowns_key(&routed)
                && play.link.boss_encounter_input(&routed))
            || play.link.animus_input(&routed)
            || play.link.arena_input(&routed)
            || play.link.exiva_input(&routed)
            || play.link.party_effects_input(&routed)
            || play.link.guild_input(&routed)
            || play.link.boss_difficulty_input(&routed)
            || play.link.boss_rewards_input(&routed)
            || play.link.chat_party_room_input(&routed)
            || play.link.cyclopedia_tracker_input(&routed)
            || play.link.world_event_input(&routed)
            || play.link.daily_boost_input(&routed)
            || play.link.mail_input(&routed)
            || play.link.house_podium_input(&routed)
            || play.link.house_rotation_input(&routed)
            || play.link.house_furniture_input(&routed)
            || play.link.training_input(&routed)
            || play.link.guild_bank_input(&routed)
            || play.link.timed_item_input(&routed)
            || play.link.premium_overview_input(&routed)
            || play.link.character_info_input(&routed)
            || play.link.raid_schedule_input(&routed)
            || play.link.guild_war_input(&routed)
            || play.link.guildhall_input(&routed)
            || play.link.house_move_out_input(&routed)
            || play.link.house_transfer_input(&routed)
            || play.link.hireling_input(&routed)
            || play.link.hotkey_equip_input(&routed)
            || play.link.potion_durations_input(&routed)
        {
            return;
        }
        if play.link.prey_input(&routed) {
            return;
        }
        if oteryn_client_ui::highscores::open_key(&routed) {
            self.controls.highscores_generation = Some(generation);
            play.link
                .highscores_request(oteryn_client_ui::highscores::Action::Refresh);
            return;
        }
        if play.link.imbue_input(&routed) {
            return;
        }
        if oteryn_client_ui::market::coin_open_key(&routed) {
            play.link.open_coin_market();
            return;
        }
        if oteryn_client_ui::market::open_key(&routed) {
            play.link.market_set_open(true);
            return;
        }
        if oteryn_client_ui::player_report::open_key(&routed) {
            play.link.rule_violation_open("");
            return;
        }
        if oteryn_client_ui::party::open_key(&routed) {
            play.link.open_party();
            return;
        }
        if oteryn_client_ui::task_board::open_key(&routed) {
            play.link
                .task_board_set_open(!play.link.task_board_panel().open);
            return;
        }
        if play.link.weapon_proficiency_input(&routed) {
            return;
        }
        if crate::input::store_key(&routed) {
            play.link.store_set_open(!play.link.store_panel().open);
            return;
        }
        if oteryn_client_ui::minimap::create_mark_key(&routed) && play.view.minimap_input(&routed) {
            return;
        }
        play.view.cursor_spell_input(&routed);
        if !play.view.diagonal_movement_input(&play.link, &routed)
            && let Some(direction) = crate::input::arrow_step(&routed)
        {
            play.view.arrow(direction);
        }
        for spell in spells {
            play.view.cast(spell);
        }
        if crate::input::logout_key(&routed) {
            if self.settings.preferences.miscellaneous.confirm_logout {
                self.request_logout_confirmation();
            } else {
                play.view.logout();
            }
            return;
        }
        let suppress_primary = play
            .view
            .world_pointer_input(&play.link, &routed)
            .unwrap_or(true);
        play.view.observe_bar_pointer(&play.link);
        let mut context = None;
        for click in clicks {
            if let Some(modifier) = click.shortcut {
                if let Some(tile) = play.view.click_tile(click.x, click.y) {
                    play.view.modifier_click(tile, &play.link, modifier);
                }
                continue;
            }
            if click.secondary && play.view.secondary_click_cancels_targeting(&play.link) {
                continue;
            }
            if suppress_primary && !click.secondary {
                continue;
            }
            if let Some(tile) = play.view.click_tile(click.x, click.y) {
                if click.secondary {
                    if !play.view.gameplay_secondary_click(&play.link) {
                        context = Some(tile);
                        self.controls.mouse.context_closed();
                        break;
                    }
                } else if let Err(error) = play.view.primary_click(tile, &play.link) {
                    self.status = format!("Scene input failed: {error}");
                }
            }
        }
        if let Some(tile) = context {
            self.request_world_context(tile);
        }
    }

    pub(super) fn request_toolbar_map(&mut self) {
        self.controls.satellite = true;
    }
    pub(super) fn request_toolbar_house(&mut self) {
        self.controls.house = Some(self.generation());
    }
    pub(super) fn take_house_request(&mut self) -> bool {
        self.controls.house.take() == Some(self.generation())
    }

    /// True once per Ctrl+Alt+Z that reached the world unblocked and unfocused.
    pub(super) fn take_satellite_request(&mut self) -> bool {
        std::mem::take(&mut self.controls.satellite)
    }

    /// A shared popup may consume the OS release; clear its retained pointer state.
    pub(super) fn pointer_context_closed(&mut self) {
        self.controls.mouse.context_closed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_preference_utilities_reach_settings_owner()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::play::{CustomAction, UtilityAction};
        use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers};
        let mut owner = HostOwner::new(None)?;
        let (link, _commands, _outbox) = crate::play::play_channel();
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        link.use_shared_frontend();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        let mut preferences = owner.settings.preferences.clone();
        preferences.add_hotkey_profile("Zeta")?;
        owner.settings.apply(preferences);
        for action in [
            UtilityAction::ToggleCreatureHuds,
            UtilityAction::ToggleFpsLagIndicator,
            UtilityAction::NextHotkeyPreset,
        ] {
            let before = owner.settings.preferences.clone();
            let mut preferences = before.clone();
            preferences.custom_hotkeys.remove("CTRL+SHIFT+F9");
            preferences.assign_custom_hotkey("CTRL+SHIFT+F9", CustomAction::Utility { action })?;
            owner.settings.apply(preferences);
            owner.route_input(
                &[NormalizedInputEvent::Key {
                    code: KeyCode::new(66)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                    repeat: false,
                }],
                false,
            );
            let after = &owner.settings.preferences;
            match action {
                UtilityAction::ToggleCreatureHuds => {
                    assert_eq!(after.creature_bars, !before.creature_bars)
                }
                UtilityAction::ToggleFpsLagIndicator => {
                    assert_ne!(after.performance.show_fps, before.performance.show_fps)
                }
                UtilityAction::NextHotkeyPreset => assert_ne!(
                    after.hotkey_contexts.selected,
                    before.hotkey_contexts.selected
                ),
                _ => return Err("unexpected utility".into()),
            }
            owner.route_input(&[NormalizedInputEvent::DeviceLost], false);
        }
        Ok(())
    }
    use crate::play::{PlayView, play_channel};
    use oteryn_input_actions::{
        ButtonState, Modifiers, MouseButton, PointerCoordinate, PointerDelta, PointerMotion,
        PointerPosition,
    };
    use oteryn_renderer::TileCoord;

    fn install(host: &mut HostOwner, mut view: PlayView, link: crate::play::PlayLink) {
        view.use_shared_frontend();
        link.use_shared_frontend();
        host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        host.phase = crate::bevy_shell::owner::Phase::Playing;
    }

    #[test]
    fn autowalk_bevy_left_ground_release_sends_native_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut host = HostOwner::new(None)?;
        let view = PlayView::new(TileCoord::new(100, 100), -3, &[])?;
        let tile = crate::input::click_tile(view.scene().view(), 48.0, 48.0).ok_or("map tile")?;
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_target_walk_selected();
        install(&mut host, view, link);
        host.route_input(
            &[
                NormalizedInputEvent::PointerMoved {
                    position: PointerPosition::new(
                        PointerCoordinate::new(48)?,
                        PointerCoordinate::new(48)?,
                    ),
                    motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
                },
                NormalizedInputEvent::MouseButton {
                    button: MouseButton::PRIMARY,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
        host.route_input(
            &[NormalizedInputEvent::MouseButton {
                button: MouseButton::PRIMARY,
                state: ButtonState::Released,
                modifiers: Modifiers::NONE,
            }],
            false,
        );
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::TargetWalk(oteryn_session::ActorPosition {
                x: tile.x,
                y: tile.y,
                floor: -3
            })
        );
        assert!(commands.try_recv().is_err());
        Ok(())
    }
    #[test]
    fn autowalk_bevy_wasd_and_arrows_cancel_before_manual_step()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::KeyCode;
        for code in [
            KeyCode::ARROW_UP,
            KeyCode::ARROW_RIGHT,
            KeyCode::ARROW_DOWN,
            KeyCode::ARROW_LEFT,
            KeyCode::new(26)?,
            KeyCode::new(7)?,
            KeyCode::new(22)?,
            KeyCode::new(4)?,
        ] {
            let mut host = HostOwner::new(None)?;
            let mut prefs = host.settings.preferences.clone();
            prefs.controls.movement = crate::play::MovementKeys::ArrowsAndWasd;
            host.settings.apply(prefs);
            let mut view = PlayView::new(TileCoord::new(100, 100), -3, &[])?;
            let (link, mut commands, outbox) = play_channel();
            outbox.bevy_target_walk_selected();
            outbox.battle_publish(
                Some(oteryn_session::ActorCombatState {
                    target: Some(oteryn_session::EntityRef {
                        identity: [7; 16],
                        generation: 1,
                    }),
                    modes: oteryn_session::FightModes::DEFAULT,
                    in_fight: true,
                }),
                Some(-3),
                &[],
            );
            view.primary_click(TileCoord::new(98, 100), &link)?;
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                commands.try_recv()?,
                crate::combat_input::PlayCommand::TargetWalk(_)
            ));
            view.primary_click(TileCoord::new(97, 100), &link)?;
            install(&mut host, view, link);
            host.route_input(
                &[NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::NONE,
                    repeat: false,
                }],
                false,
            );
            let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()?
            else {
                return Err("stop before step".into());
            };
            assert_eq!(stop.payload, [16, 1]);
            let play = host.play.as_mut().ok_or("play")?;
            play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                commands.try_recv()?,
                crate::combat_input::PlayCommand::Step(_)
            ));
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }
    #[test]
    fn manual_turn_keeps_the_active_attack() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::{KeyCode, Modifier};
        for (name, modifier) in [
            ("CTRL", Modifier::Control),
            ("ALT", Modifier::Alt),
            ("SHIFT", Modifier::Shift),
        ] {
            let mut host = HostOwner::new(None)?;
            let mut prefs = host.settings.preferences.clone();
            prefs.rotation_modifier = name.into();
            host.settings.apply(prefs);
            let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
            view.set_preferences(host.settings.preferences.clone())?;
            let (link, mut commands, outbox) = play_channel();
            outbox.bevy_target_walk_selected();
            outbox.battle_publish(
                Some(oteryn_session::ActorCombatState {
                    target: Some(oteryn_session::EntityRef {
                        identity: [7; 16],
                        generation: 1,
                    }),
                    modes: oteryn_session::FightModes::DEFAULT,
                    in_fight: true,
                }),
                Some(-7),
                &[],
            );
            install(&mut host, view, link);
            host.route_input(
                &[NormalizedInputEvent::Key {
                    code: KeyCode::ARROW_UP,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(modifier),
                    repeat: false,
                }],
                false,
            );
            let crate::combat_input::PlayCommand::ServerMechanic(turn) = commands.try_recv()?
            else {
                return Err("turn must preserve combat".into());
            };
            assert_eq!(turn.payload, [26, 2, 8, 1]);
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "controls/parity_tests.rs"]
mod parity_tests;
