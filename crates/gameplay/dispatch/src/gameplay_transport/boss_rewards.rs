//! Physical personal boss reward chest commands. The client names only its retained death
//! and item ordinal; Character, scope, Content and the reachable chest come from the owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession, guild_intent::GuildReply};
use crate::durability::reward_claim_mint::boss_reward::{
    BossRewardCollectRequest, BossRewardCollection, BossRewardCollectionStatus, BossRewardSelection,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::boss_rewards as wire;

#[path = "boss_rewards/physical.rs"]
mod physical;
#[cfg(test)]
#[path = "boss_rewards/tests.rs"]
mod tests;

enum Request {
    Browse(wire::BrowseRequest),
    Collect(BossRewardSelection),
}

fn decode(kind: u32, payload: &[u8]) -> Option<Request> {
    match kind {
        wire::COMMAND_TYPE_BOSS_REWARD_BROWSE => wire::decode_browse_request(payload)
            .ok()
            .map(Request::Browse),
        wire::COMMAND_TYPE_BOSS_REWARD_TAKE => {
            wire::decode_take_request(payload).ok().map(|request| {
                Request::Collect(BossRewardSelection::Item {
                    death: request.death,
                    ordinal: request.ordinal,
                })
            })
        }
        wire::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL => wire::decode_collect_all_request(payload)
            .ok()
            .map(|request| {
                Request::Collect(request.death.map_or(BossRewardSelection::All, |death| {
                    BossRewardSelection::Group { death }
                }))
            }),
        _ => None,
    }
}

fn status(value: BossRewardCollectionStatus) -> wire::BossRewardStatus {
    match value {
        BossRewardCollectionStatus::Collected => wire::BossRewardStatus::Ok,
        BossRewardCollectionStatus::Empty => wire::BossRewardStatus::Empty,
        BossRewardCollectionStatus::Expired => wire::BossRewardStatus::Expired,
        BossRewardCollectionStatus::NoRoom => wire::BossRewardStatus::NoRoom,
        BossRewardCollectionStatus::Rejected => wire::BossRewardStatus::Rejected,
        BossRewardCollectionStatus::ContentUnavailable => {
            wire::BossRewardStatus::ContentUnavailable
        }
    }
}

fn collected(result: BossRewardCollection) -> GuildReply {
    wire::encode_collect_reply(&wire::CollectReply {
        status: status(result.status),
        moved: result.moved,
        remaining: result.remaining,
    })
    .map_or(GuildReply::Unavailable, GuildReply::Payload)
}

fn refused(request: &Request, status: wire::BossRewardStatus) -> GuildReply {
    let bytes = match request {
        Request::Browse(request) => wire::encode_browse_reply(&wire::BrowseReply {
            status,
            death: request.death,
            expires_at_unix_ms: 0,
            rows: vec![],
            more: false,
        }),
        Request::Collect(_) => wire::encode_collect_reply(&wire::CollectReply {
            status,
            moved: 0,
            remaining: 0,
        }),
    };
    bytes.map_or(GuildReply::Unavailable, GuildReply::Payload)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn boss_rewards_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        let (Some(actor), Some(fence), Some(request), Ok(command_id)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            decode(kind, payload),
            CommandId::new(id),
        ) else {
            return GuildReply::Rejected;
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_BOSS_REWARDS_V1)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        let command = CommandRef::new(admitted.game_session_id, command_id);
        let Some(permit) = self.spell_lane_permit().await else {
            return refused(&request, wire::BossRewardStatus::Busy);
        };
        let Some(generation_native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return refused(&request, wire::BossRewardStatus::ContentUnavailable);
        };
        let mut collection = match request {
            Request::Collect(selection) => {
                let mut intent = kind.to_be_bytes().to_vec();
                intent.extend_from_slice(payload);
                let candidate = BossRewardCollectRequest {
                    command,
                    selection,
                    intent,
                    source_placement: String::new(),
                    content_revision: format!(
                        "sha256:{}",
                        generation_native
                            .source_digest()
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ),
                    ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                        .into(),
                    sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
                };
                match self
                    .root
                    .collect_boss_rewards(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        candidate.clone(),
                        &super::item_ref_admission::NativeItemWeights(generation_native),
                        true,
                    )
                    .await
                {
                    Ok(Some(receipt)) => return collected(receipt),
                    Ok(None) => Some(candidate),
                    Err(_) => return GuildReply::Unavailable,
                }
            }
            Request::Browse(_) => None,
        };
        if !self
            .states
            .player_trades
            .physical_service(admitted.game_session_id)
        {
            return refused(&request, wire::BossRewardStatus::Busy);
        }
        if !physical::current_session(self, fence).await {
            return refused(&request, wire::BossRewardStatus::Rejected);
        }
        let (native, index, source) = {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if admitted.world_id != binding.world_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return refused(&request, wire::BossRewardStatus::Rejected);
            }
            let states = self.states.spell_states.lock().await;
            let Some(native) = super::equipment_move::native_for_move(
                &runtime,
                self.active_generation,
                states.is_dead(actor),
                states.has_pending_spell_commit(actor, admitted.game_session_id),
            ) else {
                return refused(&request, wire::BossRewardStatus::Busy);
            };
            let Some(index) =
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return refused(&request, wire::BossRewardStatus::ContentUnavailable);
            };
            let source = super::bundle_map_position::bundle_map_position(
                &mut runtime,
                self.movement_cells,
                actor,
            )
            .as_ref()
            .and_then(|at| physical::reward_chest(at, index));
            (native, index, source)
        };
        let Some(source) = source else {
            return refused(&request, wire::BossRewardStatus::OutOfReach);
        };
        if let Some(candidate) = collection.as_mut() {
            candidate.source_placement = format!("oteryn:reward-chest.{source}");
            return match self
                .root
                .collect_boss_rewards(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    candidate.clone(),
                    &super::item_ref_admission::NativeItemWeights(native),
                    false,
                )
                .await
            {
                Ok(Some(result)) => collected(result),
                Ok(None) => GuildReply::Unavailable,
                Err(_) => GuildReply::Unavailable,
            };
        }
        let Request::Browse(browse) = request else {
            return GuildReply::Rejected;
        };
        let page = match self
            .root
            .read_boss_reward_items(
                self.character,
                self.holder,
                fence,
                browse.death,
                browse.after_ordinal,
                native.source_digest(),
            )
            .await
        {
            Ok(Some(page)) => page,
            Ok(None) => return refused(&Request::Browse(browse), wire::BossRewardStatus::Rejected),
            Err(_) => return GuildReply::Unavailable,
        };
        // A slow read must not publish current private handles to a displaced connection.
        if !physical::current_session(self, fence).await
            || self.runtime.lock().await.owner_fence().is_err()
        {
            return refused(&Request::Browse(browse), wire::BossRewardStatus::Rejected);
        }
        let rows = page
            .items
            .iter()
            .map(|item| {
                Some(wire::RewardRow {
                    ordinal: item.ordinal,
                    definition_reference: super::item_ref_admission::definition_ref(
                        index,
                        &item.definition,
                    )?
                    .get(),
                    quantity: item.quantity,
                })
            })
            .collect::<Option<Vec<_>>>();
        let Some(rows) = rows else {
            return refused(
                &Request::Browse(browse),
                wire::BossRewardStatus::ContentUnavailable,
            );
        };
        wire::encode_browse_reply(&wire::BrowseReply {
            status: status(page.status),
            death: page.death,
            expires_at_unix_ms: page.expires_at_unix_ms,
            rows,
            more: page.more,
        })
        .map_or(GuildReply::Unavailable, GuildReply::Payload)
    }
}

macro_rules! authority_methods {
    () => {
        fn boss_rewards_command(
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
        async fn boss_rewards_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.boss_rewards_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
