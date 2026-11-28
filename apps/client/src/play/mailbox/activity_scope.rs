//! Invalidate source actions before presenting a committed activity destination.
#[cfg(test)]
use super::lock;
use super::{Mailbox, Pushed, SessionEvent};

impl Mailbox {
    pub(in crate::play) fn cancel_activity_requests(&mut self) {
        if let Some(cancel) = self.channel_probe_cancel.take() {
            cancel.cancel();
            self.channel_probe_refreshed = None;
        }
        self.logout_requested = None;
        self.reset_wire_frame();
        self.chat.scope = self.chat.scope.wrapping_add(1);
        // Every queued command belonged to the source namespace, including panel queries.
        // Release their controls without inventing success or retrying a mutation.
        self.bar_actions.item_cooldown_read = None;
        self.bar_actions.deferred_item_read = None;
        self.bar_actions.item_cooldowns.clear();
        self.bar_actions.item_cooldown_rejected.clear();
        self.battle.pending = false;
        self.battle.pending_command = None;
        self.spellbook_details.pending = None;
        self.server_info.pending.clear();
        self.quest_info_revision = None;
        self.quest.titles.clear();
        self.spell_catalog.info.clear();
        self.spellbook_details.info.clear();
        self.chat.pending = false;
        self.quest_pending = false;
        self.animus.pending = Default::default();
        self.arena.pending = Default::default();
        self.boss_capture = Default::default();
        self.boss_encounter.pending = Default::default();
        self.boss_encounter.pending_catalogue = None;
        self.boss_encounter.catalogue_query = None;
        self.boss_encounter.catalogue = None;
        self.boss_encounter.boss_names.clear();
        self.boss_encounter.slots = None;
        self.boss_encounter.pending_kind = 0;
        self.chat_public.pending = Default::default();
        self.reward_reset = None;
        self.daily_reward.pending = Default::default();
        self.containers.pending = Default::default();
        self.depot.pending = Default::default();
        self.cyclopedia_character.pending = Default::default();
        self.cyclopedia_character.own_vocation_pending = false;
        self.cyclopedia_character.own_vocation_id = None;
        self.cyclopedia_character.augments = Default::default();
        let blessing_details = self.cyclopedia_character.blessings.details_selected;
        self.cyclopedia_character.blessings = Default::default();
        self.cyclopedia_character.blessings.details_selected = blessing_details;
        self.properties.cyclopedia_map.atlas = None;
        self.properties.cyclopedia_map.atlas_query = Default::default();
        self.cyclopedia_tracker.pending = Default::default();
        self.cyclopedia_tracker.queued = Default::default();
        self.world_event.pending = Default::default();
        self.daily_boost.pending = Default::default();
        self.highscores.pending = Default::default();
        self.depot_search.pending = Default::default();
        self.hotkey_equip.pending = Default::default();
        self.bar_actions.item_cooldown_read = None;
        self.bar_actions.deferred_item_read = None;
        self.bar_actions.item_cooldowns.clear();
        self.bar_actions.item_cooldown_rejected.clear();
        self.potion_durations.pending = Default::default();
        self.item_forge.scope_changed();
        self.store.pending = Default::default();
        self.sanctions.pending = false;
        self.tutorial_vocation.pending = false;
        self.tutorial_vocation.observation = None;
        self.weapon_proficiency.invalidate_observation();
        self.achievements.pending = Default::default();
        self.wheel.pending = Default::default();
        self.wheel.gems.invalidate();
        self.wheel.diagram.invalidate();
        self.exiva.pending = Default::default();
        self.mechanics_status.pending = Default::default();
        self.condition_status.pending = Default::default();
        self.rest_status.pending = Default::default();
        self.battle_instinct_status.pending = Default::default();
        self.skill_progress.pending = Default::default();
        self.party_effects.pending = Default::default();
        self.loyalty_skills.pending = Default::default();
        self.stash.pending = Default::default();
        self.guild.pending = Default::default();
        self.task_board.pending = Default::default();
        self.task_board.fights = Default::default();
        self.task_board.soulseals_open = false;
        self.npc_shop.pending = Default::default();
        self.npc_shop.live.pending = false;
        self.npc_shop.live.weight_pending = None;
        self.npc_shop.live.weights.clear();
        self.dialogue.pending = false;
        self.prey.pending = Default::default();
        self.imbue.invalidate_item_scope();
        self.player_trade.pending = Default::default();
        self.player_trade.items.pending = false;
        self.vip.pending = Default::default();
        self.party.pending = Default::default();
        self.party_notice_refresh.cancel_query();
        self.hireling.pending = Default::default();
        self.hireling_management.pending = Default::default();
        self.market.pending = Default::default();
        self.market.details = Default::default();
        self.market.hand_pending = None;
        self.market.hand_positions.clear();
        self.market.funds_pending = false;
        self.market.funds = None;
        self.chat_private.pending = Default::default();
        self.boss_rewards.pending = Default::default();
        self.discovery.loot.pending = Default::default();
        self.discovery.inspection.pending = Default::default();
        self.discovery.inspection.invalidate_catalogue();
        self.properties.cyclopedia_map.region_atlas = None;
        self.properties.cyclopedia_map.atlas_after = 0;
        self.properties.cyclopedia_map.map = None;
        self.properties.cyclopedia_map.summary.clear();
        self.discovery.leader.pending = Default::default();
        self.discovery.teams.pending = Default::default();
        self.discovery.history.pending = Default::default();
        self.properties.appearance.riding_after_apply = None;
        self.properties.appearance.committed_riding = None;
        self.properties.appearance.pending = Default::default();
        self.properties.charm_progression.pending = Default::default();
        self.properties.charm_progression.catalogue = Default::default();
        self.properties.charm_progression.charm_browser = Default::default();
        self.properties.charm_progression.reply = None;
        self.properties.charm_progression.summary.clear();
        self.properties.charm_progression.feedback.clear();
        self.properties.character_info.pending = Default::default();
        self.properties.raid_schedule.pending = Default::default();
        self.properties.container_sort.pending = Default::default();
        self.properties.cyclopedia_map.pending = Default::default();
        self.properties.cyclopedia_map.invalidate_atlas();
        self.properties.guild_war.pending = Default::default();
        self.properties.party_skull.pending = Default::default();
        self.properties.guildhall.pending = Default::default();
        self.properties.house.invalidate();
        self.properties.house_move_out.pending = Default::default();
        self.properties.house_transfer.pending = Default::default();
        self.properties.staff.pending = Default::default();
        self.properties.player_report.pending = Default::default();
        self.properties.pvp_expert.pending = Default::default();
        self.properties.mail.pending = Default::default();
        self.properties.house_furniture_wrap.pending = Default::default();
        self.properties.house_podium.pending = Default::default();
        #[cfg(bevy_shell)]
        {
            self.properties.expiry_pending = None;
            self.properties.expiry_reply = None;
        }
        self.properties.timed_item.pending = Default::default();
        self.properties.training.pending = Default::default();
        self.properties.guild_bank.pending = Default::default();
    }
}

impl Mailbox {
    pub(super) fn activity_scope_events(&mut self, events: &[SessionEvent]) {
        use oteryn_session::activity_scope::ActivityScope;
        let receipt =
            crate::play::feature_0117_boss_encounter::protocol::activity_scope_receipt(events);
        let Some(receipt) = receipt else { return };
        // Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:79-106
        // moves the whole staged group.
        // Discard origin targets and feedback before applying fresh destination snapshots.
        self.bar_actions.scope = crate::play::feature_0017_attack::view::next_edit();
        self.capture_observations = Default::default();
        self.analyser.pending_drop_groups.clear();
        self.analyser.pending_drop_values.clear();
        self.before = Pushed::default();
        self.pushed = Pushed {
            scope_changed: true,
            ..Default::default()
        };
        self.stepped = None;
        self.combat.clear();
        self.logout_refused = None;
        self.world_context.pending = false;
        self.world_context.targeting = None;
        self.world_context.rune_targeting = false;
        self.item_movement = Default::default();
        self.inventory.apply(None);
        self.dialogue.target = None;
        self.dialogue.open = false;
        self.npc_shop.live.conversation = None;
        self.npc_shop.live.window = Default::default();
        self.npc_shop.live.inventory = None;
        self.npc_shop.live.containers = Default::default();
        self.player_trade.offer = None;
        self.player_trade.partner = None;
        self.player_trade.state = None;
        self.player_trade.items = Default::default();
        self.containers.apply(None);
        self.appearance_state.clear();
        self.target_walk.pending = false;
        self.target_walk.response = None;
        oteryn_client_ui::target_move::cancel(&mut self.target_walk);
        self.cancel_activity_requests();
        // Destination ChatLog sequences and notices start over at the handoff.
        self.chat.console.reset_extensions();
        self.activity_reset_commands = true;
        self.map = None;
        self.action_feedback = Some(Some(match receipt.scope {
            ActivityScope::Instance { .. } => {
                tr!("Entered activity instance. Use the exit to return.")
            }
            ActivityScope::Channel { .. } => tr!("Returned to the channel."),
        }));
        self.hud_resource =
            crate::play::resource_hud::MailboxState::new(self.cyclopedia_character.selected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::feature_local_spell_action_bar::view::{
        BarAction, ItemCooldownRead, PassiveAbility,
    };

    #[test]
    fn handoff_releases_abandoned_requests_and_activations() {
        let (link, _commands, outbox) = super::super::play_channel();
        let mut mailbox = lock(&outbox.0);
        mailbox.battle.pending = true;
        mailbox.battle.pending_command = Some(crate::combat_input::CombatCommand::Stop);
        mailbox.spellbook_details.pending = Some(Default::default());
        mailbox.chat.pending = true;
        mailbox.market.hand_pending = Some(42);
        mailbox.market.funds_pending = true;
        mailbox.market.funds = Some((10, 20));
        mailbox.market.hand_positions.insert(42, Some(2));
        mailbox.npc_shop.live.weight_pending = Some(oteryn_client_ui::npc_shop::Quote {
            npc: oteryn_session::EntityRef {
                identity: [9; 16],
                generation: 2,
            },
            revision: 7,
            offer_index: 0,
            item: 42,
            side: oteryn_client_ui::npc_shop::NpcTradeSide::Buy,
            count: 1,
            price: 12,
        });
        mailbox.npc_shop.live.weights.insert(42, Some(100));
        let read = || ItemCooldownRead {
            slot: std::num::NonZeroU32::MIN,
            action: BarAction::Passive {
                ability: PassiveAbility::BattleInstinct,
            },
            edit: 0,
            definitions: vec![],
            activate: true,
            ready: false,
            cursor: None,
        };
        mailbox.bar_actions.item_cooldown_read = Some(read());
        mailbox.bar_actions.deferred_item_read = Some(read());
        mailbox.cancel_activity_requests();
        assert!(!mailbox.battle.pending);
        assert!(mailbox.battle.pending_command.is_none());
        assert!(mailbox.spellbook_details.pending.is_none());
        assert!(!mailbox.chat.pending);
        assert!(mailbox.market.hand_pending.is_none());
        assert!(!mailbox.market.funds_pending);
        assert!(mailbox.market.funds.is_none());
        assert!(mailbox.market.hand_positions.is_empty());
        assert!(mailbox.npc_shop.live.weight_pending.is_none());
        assert!(mailbox.npc_shop.live.weights.is_empty());
        drop(mailbox);
        assert!(!link.bar_activation_pending());
    }
}

#[cfg(test)]
#[test]
fn handoff_releases_hud_stamina_query_without_deselecting_it()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
    };
    let (link, mut commands, outbox) = super::play_channel();
    outbox.hud_stamina_initialize(true);
    outbox.cyclopedia_character_initialize(true);
    let mut view = crate::play::tests::view()?;
    view.enable_resource_hud(true);
    view.sync_resource_hud(&link);
    assert_eq!(commands.try_recv()?, PlayCommand::HudStamina);
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut successor = id;
    successor[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: successor,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117, 199],
        requires_ack: false,
    })
    .map_err(|error| format!("{error:?}"))?;
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        },
    )]);
    assert!(outbox.death_command(&mut commands).is_err());
    let mut destination = crate::play::tests::view()?;
    destination.enable_resource_hud(true);
    destination.sync_resource_hud(&link);
    assert_eq!(commands.try_recv()?, PlayCommand::HudStamina);
    Ok(())
}

#[cfg(test)]
#[test]
fn logout_can_be_requested_after_handoff_discards_the_queued_command() {
    let (link, mut commands, outbox) = super::play_channel();
    lock(&outbox.0).activity_reset_commands = true;
    link.request_logout();
    assert!(outbox.death_command(&mut commands).is_err());
    link.request_logout();
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Logout)
    ));
}

#[cfg(test)]
#[test]
fn pending_scope_presentation_does_not_accept_a_logout_it_will_discard() {
    let (link, mut commands, outbox) = super::play_channel();
    {
        let mut mailbox = lock(&outbox.0);
        mailbox.scope_pending = true;
        mailbox.scope_session = Some([1; 16]);
    }
    link.request_logout();
    assert!(commands.try_recv().is_err());
    link.scope_presented([1; 16]);
    link.request_logout();
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Logout)
    ));
}
