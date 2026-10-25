//! PARITY-PROGRESSION-QUERIES-1: Bestiary and charm progression (capability 67, command 84) on
//! the real admitted connection. The query and the three mutations go through the native charm
//! port: the same gameplay fence, revision slot and receipts as CHARM-3, and the gold fee owner
//! for a remove. The Bestiary races and their charm points are the canonical Bestiary source,
//! the catalogue is the resident imported one, and both views are read after the command.
use std::sync::LazyLock;

use super::charm::CharmProgressionPort;
use super::charm::native::{CharmConnectionContent, NativeCharmProgressionPort};
use super::guild_intent::GuildReply;
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::ProjectEvidenceLimits;
use crate::domain::bestiary::BestiaryRace;
use crate::durability::charm_state::{
    BestiaryCharmEntry, BestiaryCharmFacts, CharmCommandOccurrence, CharmRemoveFee,
};
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::foundation::{CommandId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::charm::{CharmAssignIntent, CharmUnlockStageIntent};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};

/// The canonical Bestiary races and the charm facts over them; `None` when the source does not
/// decode, and the capability is then not offered.
struct Bestiary {
    races: Vec<BestiaryRace>,
    facts: BestiaryCharmFacts,
}

fn bestiary() -> Option<&'static Bestiary> {
    static BESTIARY: LazyLock<Option<Bestiary>> = LazyLock::new(|| {
        // The creature reward table's evidence limits for the same source.
        let rows = crate::content::canonical_bestiary_rows(ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 2_500_000,
            max_total_bytes: 8_000_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: 4096,
            max_import_records: 1,
            max_reimport_states: 1,
        })
        .ok()?;
        let facts = BestiaryCharmFacts::new(rows.iter().map(|(race, charm_points)| {
            BestiaryCharmEntry {
                race: race.clone(),
                charm_points: *charm_points,
            }
        }))
        .ok()?;
        Some(Bestiary {
            races: rows.into_iter().map(|(race, _)| race).collect(),
            facts,
        })
    });
    BESTIARY.as_ref()
}

/// The canonical Bestiary races in wire index order (index = position + 1).
#[cfg(test)]
pub(super) fn canonical_races() -> Option<&'static [BestiaryRace]> {
    bestiary().map(|b| b.races.as_slice())
}

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let id = wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1;
    let mut offers: Vec<_> = base.iter().copied().filter(|c| c.id != id).collect();
    if available {
        let at = offers.partition_point(|c| c.id < id);
        offers.insert(at, OfferedCapability { id, requires: &[] });
    }
    offers
}

/// The remove's coin facts: the current coin definitions and the main backpack's.
fn fee_change(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &crate::foundation::ChannelRuntimeV1,
    backpack: Option<&crate::durability::item_transfer::CharacterBackpack>,
    item_instance_ids: [[u8; 16]; 2],
) -> Option<FeeChangeFacts> {
    let (gold, platinum, crystal) = super::npc_command::bank_coin_definitions(admission, runtime)?;
    let index = super::item_ref_admission::pinned_index(runtime, admission.active_generation)?;
    let fallback = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2854".into(),
        revision_ref: index.revision("oteryn:item.tibia.i2854")?.into(),
    };
    let native = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)?;
    let definition = backpack.map_or(&fallback, |b| &b.backpack.definition);
    let backpack = super::item_ref_admission::generation_item_facts(native, definition).ok()?;
    Some(FeeChangeFacts {
        item_instance_ids,
        gold,
        platinum,
        crystal,
        backpack,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn charm_progression_ready(&self) -> bool {
        bestiary().is_some()
    }

    pub(super) async fn charm_progression_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        if kind != wire::COMMAND_TYPE_CHARM_PROGRESSION
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(actor), Some(fence), Some(bestiary)) =
            (admitted.runtime_actor, admitted.item_fence, bestiary())
        else {
            return GuildReply::Rejected;
        };
        let Ok(action) = wire::decode_charm_progression_request(payload) else {
            return GuildReply::Rejected;
        };
        let Ok(id) = CommandId::new(id) else {
            return GuildReply::Rejected;
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
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id.get());
        let backpack = if matches!(action, CharmProgressionAction::Remove { .. }) {
            match self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
            {
                Ok(backpack) => backpack,
                Err(_) => return GuildReply::Unavailable,
            }
        } else {
            None
        };
        // Ownership and the fee facts under the Channel; released before the revision slot.
        let change = {
            let runtime = self.runtime.lock().await;
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
            if matches!(action, CharmProgressionAction::Remove { .. }) {
                let ids = [
                    derive(b"charm-progression:change:0"),
                    derive(b"charm-progression:change:1"),
                ];
                let Some(change) = fee_change(self, &runtime, backpack.as_ref(), ids) else {
                    return GuildReply::Rejected;
                };
                Some(change)
            } else {
                None
            }
        };
        // The catalogue binds the Character's current content revision: the catalogue and the
        // Bestiary source are embedded in this build, so every content revision serves them.
        let revision = match self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
        {
            Ok(Some(state)) => state.context.content,
            Ok(None) => return GuildReply::Rejected,
            Err(_) => return GuildReply::Unavailable,
        };
        let Ok(content) = CharmConnectionContent::new(
            revision,
            self.imported_charms.catalogue().clone(),
            bestiary.races.clone(),
            self.imported_charms,
        ) else {
            return GuildReply::Unavailable;
        };
        let Ok(mut port) = NativeCharmProgressionPort::bind(
            self.root,
            self.character,
            self.holder,
            &self.revision_sequencer,
            current,
            &content,
            bestiary.facts.clone(),
        )
        .await
        else {
            return GuildReply::Rejected;
        };
        let Ok(occurrence) =
            CharmCommandOccurrence::from_bytes(derive(b"charm-progression:occurrence"))
        else {
            return GuildReply::Rejected;
        };
        let (outcome, fee_gold) = match action {
            CharmProgressionAction::Query => (CharmProgressionOutcome::Query, 0),
            CharmProgressionAction::Unlock {
                charm,
                expected_stage,
            } => (
                CharmProgressionOutcome::Unlock(
                    port.unlock_next_stage(
                        occurrence,
                        CharmUnlockStageIntent {
                            charm,
                            expected_stage,
                        },
                    )
                    .await,
                ),
                0,
            ),
            CharmProgressionAction::Assign { charm, race } => (
                CharmProgressionOutcome::Assign(
                    port.assign(occurrence, CharmAssignIntent { charm, race })
                        .await,
                ),
                0,
            ),
            CharmProgressionAction::Remove { charm } => {
                let Some(change) = change else {
                    return GuildReply::Rejected;
                };
                let fee = CharmRemoveFee {
                    transaction_id: derive(b"charm-progression:fee"),
                    event_id: derive(b"charm-progression:event"),
                    occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
                    change,
                };
                let (disposition, fee_gold) = port.remove(occurrence, charm, fee).await;
                (CharmProgressionOutcome::Remove(disposition), fee_gold)
            }
        };
        let Ok(views) = port.views().await else {
            return GuildReply::Unavailable;
        };
        match wire::encode_charm_progression_reply(&CharmProgressionReply {
            outcome,
            fee_gold,
            bestiary: views.bestiary,
            charms: views.charms,
        }) {
            Ok(bytes) => GuildReply::Payload(bytes),
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn charm_progression_available(&self) -> bool {
            false
        }
        fn charm_progression_command(
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
        fn charm_progression_available(&self) -> bool {
            self.charm_progression_ready()
        }
        async fn charm_progression_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.charm_progression_wire(admitted, id, kind, payload)
                .await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[path = "charm_progression_tests.rs"]
mod tests;
