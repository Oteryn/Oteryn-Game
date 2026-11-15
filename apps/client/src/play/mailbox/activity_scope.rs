//! Invalidate source actions before presenting a committed activity destination.
use super::{Mailbox, Outbox, Pushed, SessionEvent, lock};

impl Mailbox {
    pub(in crate::play) fn cancel_activity_requests(&mut self) {
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
        self.quest.titles.clear();
        self.spell_catalog.info.clear();
        self.spellbook_details.info.clear();
        self.chat.pending = false;
        self.quest_pending = false;
        self.animus.pending = Default::default();
        self.arena.pending = Default::default();
        self.boss_encounter.pending = Default::default();
        self.boss_encounter.pending_catalogue = None;
        self.boss_encounter.catalogue_query = None;
        self.boss_encounter.catalogue = None;
        self.boss_encounter.boss_names.clear();
        self.boss_encounter.slots = None;
        self.boss_encounter.pending_kind = 0;
        self.chat_public.pending = Default::default();
        self.daily_reward.pending = Default::default();
        self.containers.pending = Default::default();
        self.depot.pending = Default::default();
        self.cyclopedia_character.pending = Default::default();
        self.cyclopedia_character.augments = Default::default();
        self.properties.cyclopedia_map.atlas = None;
        self.properties.cyclopedia_map.atlas_query = Default::default();
        self.cyclopedia_tracker.pending = Default::default();
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
        self.dialogue.pending = false;
        self.prey.pending = Default::default();
        self.imbue.invalidate_item_scope();
        self.player_trade.pending = Default::default();
        self.player_trade.items.pending = false;
        self.vip.pending = Default::default();
        self.party.pending = Default::default();
        self.hireling.pending = Default::default();
        self.hireling_management.pending = Default::default();
        self.market.pending = Default::default();
        self.market.details = Default::default();
        self.chat_private.pending = Default::default();
        self.boss_rewards.pending = Default::default();
        self.discovery.loot.pending = Default::default();
        self.discovery.inspection.pending = Default::default();
        self.discovery.inspection.invalidate_catalogue();
        self.discovery.inspection.invalidate_catalogue();
        self.properties.cyclopedia_map.region_atlas = None;
        self.properties.cyclopedia_map.atlas_after = 0;
        self.properties.cyclopedia_map.map = None;
        self.properties.cyclopedia_map.summary.clear();
        self.discovery.leader.pending = Default::default();
        self.discovery.teams.pending = Default::default();
        self.discovery.history.pending = Default::default();
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
        self.properties.timed_item.pending = Default::default();
        self.properties.training.pending = Default::default();
        self.properties.guild_bank.pending = Default::default();
    }
}

impl Outbox {
    pub(super) fn activity_scope_events(&self, events: &[SessionEvent]) {
        use oteryn_session::activity_scope::ActivityScope;
        let receipt =
            crate::play::feature_0117_boss_encounter::protocol::activity_scope_receipt(events);
        let Some(receipt) = receipt else { return };
        let mut mailbox = lock(&self.0);
        // Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:79-106
        // moves the whole staged group.
        // Discard origin targets and feedback before applying fresh destination snapshots.
        mailbox.bar_actions.scope = crate::play::feature_0017_attack::view::next_edit();
        mailbox.before = Pushed::default();
        mailbox.pushed = Pushed {
            scope_changed: true,
            ..Default::default()
        };
        mailbox.stepped = None;
        mailbox.combat.clear();
        mailbox.logout_refused = None;
        mailbox.world_context.pending = false;
        mailbox.world_context.targeting = None;
        mailbox.world_context.rune_targeting = false;
        mailbox.item_movement = Default::default();
        mailbox.inventory.apply(None);
        mailbox.dialogue.target = None;
        mailbox.dialogue.open = false;
        mailbox.npc_shop.live.conversation = None;
        mailbox.npc_shop.live.window = Default::default();
        mailbox.npc_shop.live.inventory = None;
        mailbox.npc_shop.live.containers = Default::default();
        mailbox.player_trade.offer = None;
        mailbox.player_trade.partner = None;
        mailbox.player_trade.state = None;
        mailbox.player_trade.items = Default::default();
        mailbox.containers.apply(None);
        mailbox.appearance_state.clear();
        mailbox.target_walk.pending = false;
        mailbox.target_walk.response = None;
        mailbox.cancel_activity_requests();
        // Destination ChatLog sequences and notices start over at the handoff.
        mailbox.chat.console.reset_extensions();
        mailbox.activity_reset_commands = true;
        mailbox.map = None;
        mailbox.action_feedback = Some(Some(match receipt.scope {
            ActivityScope::Instance { .. } => {
                tr!("Entered activity instance. Use the exit to return.")
            }
            ActivityScope::Channel { .. } => tr!("Returned to the channel."),
        }));
        let stamina_selected = mailbox.cyclopedia_character.selected;
        drop(mailbox);
        self.hud_stamina_initialize(stamina_selected);
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
