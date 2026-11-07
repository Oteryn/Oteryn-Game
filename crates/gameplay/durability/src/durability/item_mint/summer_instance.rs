//! Closed Summer2026 descendant of the existing ordinary creature MINT owner.
//! No source combat statistics, loot draws or activity lifecycle are replaced.
use super::*;
use crate::foundation::RuntimeScopeRefV1;

const SOURCES: [(&str, u32); 8] = [
    ("rakesh_moonfang", 43758),
    ("maior_domus", 54433),
    ("maior_domus_final", 54433),
    ("mimar_haffar", 54346),
    ("mimar_haffar_final", 54346),
    ("phosphorus", 54352),
    ("phosphorus_final", 54352),
    ("the_moonsnow_magnolia", 54670),
];

impl ItemMintCause {
    /// The exact native table and physical body must qualify together. These
    /// arguments come from the committed lethal owner's content-qualified row;
    /// no client topic, origin Channel or arbitrary death bytes are accepted.
    pub fn from_summer_instance_death(
        death: CreatureDeathOccurrenceKey,
        native_loot: &TypedDefinitionRef,
        native_corpse: &TypedDefinitionRef,
        output_table: TypedDefinitionRef,
        purpose_key: String,
        draw_ordinal: u32,
    ) -> Result<Self> {
        if !matches!(death.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || !source_pair(native_loot, native_corpse)
            || if purpose_key == CORPSE_MATERIALIZATION_PURPOSE_KEY {
                draw_ordinal != 0 || output_table != *native_corpse
            } else {
                output_table != *native_loot
            }
        {
            return Err(ItemMintError::AuthorityRejected);
        }
        Ok(Self {
            death: CreatureDeathKey {
                world_id: death.world_id(),
                scope: death.runtime_scope(),
                scope_ownership_generation: death.scope_ownership_generation(),
                actor_local_id: death.actor_local_id(),
                actor_local_generation: death.actor_local_generation(),
            },
            loot_table: output_table,
            purpose_key,
            draw_ordinal,
        })
    }
}
fn source_pair(loot: &TypedDefinitionRef, corpse: &TypedDefinitionRef) -> bool {
    loot.family == "Loot"
        && corpse.family == "Item"
        && loot.revision_ref == "definition-r1"
        && corpse.revision_ref == "definition-r1"
        && SOURCES.iter().any(|(creature, body)| {
            loot.production_key == format!("oteryn:loot.creature.{creature}")
                && corpse.production_key == format!("oteryn:item.tibia.i{body}")
        })
}
impl CreatureDeathKey {
    pub(super) fn typed_cause(&self) -> &'static str {
        if self.instance_bytes().is_some() {
            audit::SUMMER_INSTANCE_LOOT_MINT_TYPED_CAUSE
        } else {
            LOOT_MINT_TYPED_CAUSE
        }
    }
    pub(super) fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub(super) fn scope_key(&self) -> [u8; 33] {
        super::super::runtime_scope_assignment::assignment_scope_key(self.scope)
    }
    pub(super) fn channel_id(&self) -> Option<ChannelId> {
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => Some(channel_id),
            _ => None,
        }
    }
    pub(super) fn identity_bytes(&self) -> &[u8; 16] {
        match &self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => channel_id.as_bytes(),
            RuntimeScopeRefV1::Instance { instance_id, .. } => instance_id,
        }
    }
    pub(super) fn channel_bytes(&self) -> Option<Vec<u8>> {
        self.channel_id().map(|id| id.as_bytes().to_vec())
    }
    pub(super) fn instance_bytes(&self) -> Option<Vec<u8>> {
        match self.scope {
            RuntimeScopeRefV1::Instance { instance_id, .. } => Some(instance_id.to_vec()),
            _ => None,
        }
    }
}

pub(crate) fn audit_table_allowed(
    table: &audit::OneItemTypedDefinitionRevisionV1,
    corpse: bool,
) -> bool {
    table.revision_ref == "definition-r1"
        && SOURCES.iter().any(|(creature, body)| {
            if corpse {
                table.family == "Item"
                    && table.production_key == format!("oteryn:item.tibia.i{body}")
            } else {
                table.family == "Loot"
                    && table.production_key == format!("oteryn:loot.creature.{creature}")
            }
        })
}
