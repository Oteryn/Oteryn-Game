//! Invalidate source actions before presenting a committed activity destination.
use super::{Mailbox, Outbox, Pushed, SessionEvent, lock};

impl Mailbox {
    pub(in crate::play) fn cancel_activity_requests(&mut self) {
        // Every queued command belonged to the source namespace, including panel queries.
        // Release their controls without inventing success or retrying a mutation.
        self.quest_pending = false;
        self.animus.pending = Default::default();
        self.arena.pending = Default::default();
        self.boss_encounter.pending = Default::default();
        self.chat_public.pending = Default::default();
        self.daily_reward.pending = Default::default();
        self.containers.pending = Default::default();
        self.depot.pending = Default::default();
        self.cyclopedia_character.pending = Default::default();
        self.cyclopedia_tracker.pending = Default::default();
        self.world_event.pending = Default::default();
        self.daily_boost.pending = Default::default();
        self.highscores.pending = Default::default();
        self.depot_search.pending = Default::default();
        self.hotkey_equip.pending = Default::default();
        self.potion_durations.pending = Default::default();
        self.item_forge.pending = Default::default();
        self.store.pending = Default::default();
        self.weapon_proficiency.invalidate_observation();
        self.achievements.pending = Default::default();
        self.wheel.pending = Default::default();
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
        self.chat_private.pending = Default::default();
        self.boss_rewards.pending = Default::default();
        self.discovery.loot.pending = Default::default();
        self.discovery.inspection.pending = Default::default();
        self.discovery.leader.pending = Default::default();
        self.discovery.teams.pending = Default::default();
        self.discovery.history.pending = Default::default();
        self.properties.appearance.pending = Default::default();
        self.properties.charm_progression.pending = Default::default();
        self.properties.character_info.pending = Default::default();
        self.properties.raid_schedule.pending = Default::default();
        self.properties.container_sort.pending = Default::default();
        self.properties.cyclopedia_map.pending = Default::default();
        self.properties.guild_war.pending = Default::default();
        self.properties.party_skull.pending = Default::default();
        self.properties.guildhall.pending = Default::default();
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
        mailbox.activity_reset_commands = true;
        mailbox.map = None;
        mailbox.action_feedback = Some(Some(match receipt.scope {
            ActivityScope::Instance { .. } => {
                tr!("Entered activity instance. Use the exit to return.")
            }
            ActivityScope::Channel { .. } => tr!("Returned to the channel."),
        }));
    }
}
