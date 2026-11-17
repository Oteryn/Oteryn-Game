//! Shared boss cooldown query, composed with the admitted Character and native durability.
use super::connection::FreshAdmissionAuthority;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::boss_encounter as wire;
#[path = "boss_encounter/catalogue.rs"]
mod catalogue;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn boss_cooldowns_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> super::guild_intent::GuildReply {
        use super::guild_intent::GuildReply;
        if kind == oteryn_protocol_oteryn::bosstiary_slots::COMMAND_TYPE_BOSSTIARY_SLOTS {
            return self.bosstiary_slots_wire(admitted, id, payload).await;
        }
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        let catalogue_query = if kind == wire::COMMAND_TYPE_BOSS_COOLDOWNS {
            wire::catalogue::Query::decode(payload)
        } else {
            None
        };
        let mut catalogue_page = None;
        if id == 0
            || !matches!(
                kind,
                wire::COMMAND_TYPE_BOSS_COOLDOWNS
                    | wire::COMMAND_TYPE_BOSS_REWARD_OPEN
            )
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BOSS_ENCOUNTER_V1)
            || (kind == wire::COMMAND_TYPE_BOSS_COOLDOWNS
                && catalogue_query.is_none() && wire::decode_cooldown_query(payload).is_err())
            || (kind == wire::COMMAND_TYPE_BOSS_REWARD_OPEN
                && (wire::decode_reward_chest_query(payload).is_err()
                    || (!payload.is_empty()
                        && (!admitted.continuity.selected_capabilities.contains(
                                oteryn_protocol_oteryn::boss_rewards::CAPABILITY_BOSS_REWARDS_V1)
                            || !self.offered_capabilities().iter().any(|capability|
                                capability.id == oteryn_protocol_oteryn::boss_rewards::CAPABILITY_BOSS_REWARDS_V1)))))
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            // The admitted Channel id is retained origin metadata after an activity
            // handoff. The current exact tagged scope is carried by the item fence.
            if admitted.world_id != binding.world_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
            {
                return GuildReply::Rejected;
            }
            if let Some(query) = &catalogue_query {
                catalogue_page = catalogue::project(self, query, &runtime);
                if catalogue_page.is_none() {
                    return GuildReply::Rejected;
                }
            }
        }
        let result = if let Some(page) = catalogue_page {
            self.root
                .read_boss_catalogue_page(self.character, self.holder, fence, page)
                .await
        } else if kind == wire::COMMAND_TYPE_BOSS_REWARD_OPEN {
            self.root
                .read_boss_reward_chest_page(self.character, self.holder, fence, payload)
                .await
        } else {
            self.root
                .read_boss_cooldown_list(self.character, self.holder, fence, payload)
                .await
        };
        match result {
            Ok(Some(bytes)) => GuildReply::Payload(bytes),
            Ok(None) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn boss_cooldowns_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn boss_cooldowns_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.boss_cooldowns_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn bosstiary_slots_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> super::guild_intent::GuildReply {
        use super::guild_intent::GuildReply;
        use oteryn_protocol_oteryn::bosstiary_slots::BossSlotRequest;
        if id == 0
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(actor), Some(expected), Some(request)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            BossSlotRequest::decode(payload),
        ) else {
            return GuildReply::Rejected;
        };
        let Some(fence) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return GuildReply::Rejected;
        };
        if expected != fence.item_fence()
            || expected.game_session_id != admitted.game_session_id
            || expected.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id);
        let backpack = match self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
        {
            Ok(v) => v,
            Err(_) => return GuildReply::Unavailable,
        };
        let fee = {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if admitted.world_id != binding.world_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|p| p.control_loss.is_none())
            {
                return GuildReply::Rejected;
            }
            super::charm_progression::fee_change(
                self,
                &runtime,
                backpack.as_ref(),
                [derive(b"boss-slot:change:0"), derive(b"boss-slot:change:1")],
            )
            .map(|change| crate::durability::charm_state::CharmRemoveFee {
                transaction_id: derive(b"boss-slot:fee"),
                event_id: derive(b"boss-slot:event"),
                occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
                change,
            })
        };
        let Some(catalogue) = super::charm_progression::canonical_bosses() else {
            return GuildReply::Unavailable;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(revision) = slot.cursor(self.root, self.character).await else {
            return GuildReply::Unavailable;
        };
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision: revision,
            ..fence
        };
        match self
            .root
            .bosstiary_slots(
                self.character,
                self.holder,
                fence,
                derive(b"boss-slot:occurrence"),
                request,
                catalogue.to_vec(),
                fee,
            )
            .await
        {
            Ok(reply) => reply
                .encode()
                .map_or(GuildReply::Unavailable, GuildReply::Payload),
            Err(_) => GuildReply::Rejected,
        }
    }
}

/// Crystal data/libs/functions/monster.lua:219-221 equipment bonus rerolls; the existing partition
/// excludes unique entries, currency, consumables and creature products (W13/W22a).
pub(super) fn apply_slot_loot_bonus(
    table: &mut oteryn_gameplay_items::loot_plan::LootTableDefinition,
    percent: u16,
    death: [u8; 16],
    equipment: impl Fn(&str) -> bool,
) {
    if percent == 0
        || table.algorithm != crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm
    {
        return;
    }
    let extra = slot_bonus_rolls(percent, death, &[]);
    if extra == 0 {
        return;
    }
    let boosted_entries = oteryn_gameplay_items::loot_plan::partition_daily_boss_bonus_entries(
        &mut table.entries,
        equipment,
    );
    table.algorithm = crate::combat::LootSelectionAlgorithm::DailyBoosted {
        extra_rolls: extra,
        boosted_entries,
    };
}

pub(super) fn slot_bonus_rolls(percent: u16, death: [u8; 16], recipient: &[u8]) -> u8 {
    use sha2::{Digest, Sha256};
    // Crystal src/io/io_bosstiary.cpp:225-232 supplies the percentage and
    // src/lua/functions/creatures/player/player_functions.cpp getBossBonus applies it per player.
    let hash = Sha256::new()
        .chain_update(b"oteryn:boss-slot-roll:v1")
        .chain_update(death)
        .chain_update(recipient)
        .finalize();
    let draw = u16::from_be_bytes([hash[0], hash[1]]) % 100;
    (percent / 100 + u16::from(draw < percent % 100)).min(3) as u8
}

#[cfg(test)]
mod slot_loot_tests {
    use super::slot_bonus_rolls;

    #[test]
    fn donor_percentage_is_an_exact_per_recipient_extra_roll_multiplier() {
        let death = [9; 16];
        // These recipients straddle the deterministic 25% draw boundary, proving that one
        // personal boss death does not share a single bonus roll among all reward chests.
        assert_eq!(slot_bonus_rolls(25, death, &[6; 16]), 1);
        assert_eq!(slot_bonus_rolls(25, death, &[1; 16]), 0);
        assert_eq!(slot_bonus_rolls(100, death, &[2; 16]), 1);
        assert_eq!(slot_bonus_rolls(250, death, &[2; 16]), 2);
        assert_eq!(slot_bonus_rolls(300, death, &[2; 16]), 3);
    }
}
