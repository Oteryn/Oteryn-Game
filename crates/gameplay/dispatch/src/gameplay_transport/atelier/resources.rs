//! Separately negotiated live balances; old Atelier replies remain immutable.
use super::AtelierWireReply;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::atelier::resources as wire;

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn atelier_resources_wire(
        &self,
        admitted: AdmittedSession,
        expected: CurrentCharacterItemFence,
        payload: &[u8],
    ) -> AtelierWireReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ATELIER_RESOURCES_V1)
            || wire::decode_query(payload).is_err()
        {
            return AtelierWireReply::Rejected;
        }
        let fence = match self.current_quest_fence(admitted.game_session_id).await {
            Ok(Some(fence)) => fence,
            _ => return AtelierWireReply::Rejected,
        };
        if expected.character_id != fence.character_id
            || expected.game_session_id != fence.game_session_id
            || expected.connection_generation != fence.connection_generation
            || expected.character_lease_generation != fence.character_lease_generation
            || expected.runtime_scope != fence.runtime_scope
            || expected.scope_ownership_generation != fence.scope_ownership_generation
        {
            return AtelierWireReply::Rejected;
        }
        // This read neither acquires an ExAction budget nor creates a mutation
        // receipt. The existing durable owner fences one current resource snapshot.
        let balances = match self
            .root
            .read_character_atelier_resources(self.character, self.holder, fence)
            .await
        {
            Ok(balances) => balances,
            Err(
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::InvalidInput
                | CharacterProgressionError::MissingProgressionState
                | CharacterProgressionError::CharacterRevisionMismatch
                | CharacterProgressionError::ProgressionContextMismatch,
            ) => {
                return AtelierWireReply::Rejected;
            }
            Err(_) => return AtelierWireReply::Unavailable,
        };
        AtelierWireReply::Payload(wire::encode_reply(&wire::ResourceBalances {
            bank_gold: balances.bank_gold,
            carried_gold: balances.carried_gold,
            lesser_gems: balances.lesser_gems,
            regular_gems: balances.regular_gems,
            greater_gems: balances.greater_gems,
            basic_fragments: balances.basic_fragments,
            supreme_fragments: balances.supreme_fragments,
        }))
    }
}
