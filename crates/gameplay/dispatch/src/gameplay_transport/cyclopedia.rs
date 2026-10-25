//! Basic Cyclopedia on the real admitted connection, owner option A.
use super::guild_intent::GuildReply;
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::{CollisionClass, LogicalCell};
use crate::domain::cyclopedia::{BasicArea, MapAction, Point};
use crate::durability::DurabilityError;
use crate::durability::character_cyclopedia::{CyclopediaPremiumGate, MapRequest};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_purchase::{NpcPurchaseGate, PurchaseService};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::cyclopedia as wire;
use sha2::{Digest, Sha256};

#[path = "cyclopedia/content.rs"]
mod content;
#[path = "cyclopedia/respawn.rs"]
mod respawn;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_CYCLOPEDIA_MAP_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_CYCLOPEDIA_MAP_V1,
                requires: &[],
            },
        );
    }
    offers
}
struct Premium<'a>(Option<&'a super::spell_premium_coordinator::SpellPremiumCoordinator>);
impl CyclopediaPremiumGate for Premium<'_> {
    async fn premium(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        match self.0 {
            Some(gate) => {
                gate.authorize(tx, account, &PurchaseService::Blessing(String::new()))
                    .await
            }
            None => Ok(false),
        }
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn cyclopedia_content(&self) -> Option<content::AdmittedArea> {
        content::admit(self)
    }
    pub(super) async fn cyclopedia_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(actor), Some(fence), Some(area)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            self.cyclopedia_content(),
        ) else {
            return GuildReply::Rejected;
        };
        let Ok(intent) = wire::decode_request(kind, payload) else {
            return GuildReply::Rejected;
        };
        let Ok(id) = CommandId::new(id) else {
            return GuildReply::Rejected;
        };
        let _trade_permit = if intent.operation == 4 {
            let Some(permit) = self.spell_lane_permit().await else {
                return GuildReply::Rejected;
            };
            if !self
                .states
                .player_trades
                .physical_service(admitted.game_session_id)
            {
                return GuildReply::Rejected;
            }
            Some(permit)
        } else {
            None
        };
        let Some(current) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return GuildReply::Rejected;
        };
        if current.item_fence() != fence
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        // Settle delivery before borrowing the Character transaction. The in-transaction
        // gate still rechecks the real root Account's current entitlement; no nested DB read.
        let account = admitted.controller.map(|c| c.account_id).or_else(|| {
            self.states
                .premium_sessions
                .lock()
                .ok()
                .and_then(|s| s.account_of(admitted.game_session_id))
        });
        let premium_ready = if matches!(intent.operation, 1 | 2) {
            match (self.premium_coordinator, account) {
                (Some(gate), Some(account)) => gate.premium_gate(account).await.premium_current(),
                _ => false,
            }
        } else {
            false
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
        {
            return GuildReply::Rejected;
        }
        let Some(position) = super::bundle_map_position(&mut runtime, self.movement_cells, actor)
        else {
            return GuildReply::Rejected;
        };
        let at = Point {
            x: position.at.x,
            y: position.at.y,
            floor: position.at.floor,
        };
        let action = match intent.operation {
            0 => MapAction::Query,
            1 => MapAction::Start,
            2 => MapAction::Shuffle,
            3 => MapAction::Find,
            4 => MapAction::Donate(intent.amount),
            _ => return GuildReply::Rejected,
        };
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id.get());
        let fee = if matches!(action, MapAction::Donate(_)) {
            let Some((gold, platinum, crystal)) =
                super::npc_command::bank_coin_definitions(self, &runtime)
            else {
                return GuildReply::Rejected;
            };
            let Some(index) =
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return GuildReply::Rejected;
            };
            let Some(revision) = index.revision("oteryn:item.tibia.i2854") else {
                return GuildReply::Rejected;
            };
            let fallback = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: revision.into(),
            };
            let backpack = match self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
            {
                Ok(b) => b,
                Err(_) => return GuildReply::Unavailable,
            };
            let Some(native) = self
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
            else {
                return GuildReply::Rejected;
            };
            let definition = backpack
                .as_ref()
                .map(|b| &b.backpack.definition)
                .unwrap_or(&fallback);
            let Ok(backpack) = super::item_ref_admission::generation_item_facts(native, definition)
            else {
                return GuildReply::Rejected;
            };
            Some(FeeChangeFacts {
                item_instance_ids: [
                    derive(b"cyclopedia:change:0"),
                    derive(b"cyclopedia:change:1"),
                ],
                gold,
                platinum,
                crystal,
                backpack,
            })
        } else {
            None
        };
        let mut hash = Sha256::new();
        hash.update(b"oteryn:cyclopedia-command:v1");
        hash.update(kind.to_be_bytes());
        hash.update(payload);
        let request = MapRequest {
            command: CommandRef::new(admitted.game_session_id, id),
            action,
            binding: hash.finalize().into(),
            occurrence: derive(b"cyclopedia:occurrence"),
            bundle: area.digest,
            epoch: self.reset_epoch,
            pool: area.pool,
            at,
            fee,
            transaction: derive(b"cyclopedia:fee"),
            event: derive(b"cyclopedia:event"),
            now: super::npc_trade_command::unix_ms_now(),
        };
        match slot
            .cyclopedia(
                self.root,
                self.character,
                self.holder,
                current,
                request,
                &Premium(self.premium_coordinator.filter(|_| premium_ready)),
            )
            .await
        {
            Ok(bytes) if wire::decode_result(&bytes).is_ok() => {
                drop(runtime);
                drop(slot);
                if self.initialize_cyclopedia_respawn().await.is_err() {
                    return GuildReply::Unavailable;
                }
                GuildReply::Payload(bytes)
            }
            Ok(_) => GuildReply::Unavailable,
            Err(
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::InvalidInput
                | CharacterProgressionError::ConflictingOccurrence,
            ) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn cyclopedia_available(&self) -> bool {
            false
        }
        fn cyclopedia_command(
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
        fn cyclopedia_available(&self) -> bool {
            self.cyclopedia_content().is_some()
        }
        async fn cyclopedia_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.cyclopedia_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
