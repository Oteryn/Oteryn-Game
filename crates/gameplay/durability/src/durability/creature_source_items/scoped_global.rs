//! A registered GlobalEvent has a real scoped identity and no Creature actor.
use super::*;
use crate::foundation::{RuntimeScopeRefV1, RuntimeWorkStamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopedGlobalIdentity {
    pub(super) scope_identity: [u8; 16],
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) ownership_generation: u64,
    pub(super) node: [u8; 16],
    pub(super) node_registration_revision: u64,
    pub(super) assignment_revision: u64,
    pub(super) stamp: RuntimeWorkStamp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlobalFireAction {
    Create,
    Expire,
}
impl GlobalFireAction {
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Expire => "expire",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopedGlobalGroundCause {
    pub(super) action: GlobalFireAction,
    pub(super) identity: ScopedGlobalIdentity,
    pub(super) source_items: Vec<TypedDefinitionRef>,
    pub(super) definition: TypedDefinitionRef,
    pub(super) occurrence: String,
    pub(super) content: [u8; 32],
    pub(super) client: [u8; 32],
    pub(super) activation: u64,
    pub(super) map: [u8; 32],
    pub(super) frame: [u8; 32],
    pub(super) body: [u8; 32],
    pub(super) cast: [u8; 32],
}
pub(super) struct PreparedGlobalGroundOperations {
    pub(super) scope_identity: [u8; 16],
    pub(super) body: [u8; 32],
    pub(super) cast: [u8; 32],
    pub(super) operations: Vec<SpellItemOperation>,
}
#[derive(Clone, Copy)]
pub(super) enum ScopedGroundCauseRef<'a> {
    Creature(&'a CreatureGroundCause),
    Global(&'a ScopedGlobalGroundCause),
}
#[derive(Clone, Copy)]
pub(super) enum ScopedGroundPreparedRef<'a> {
    Creature(&'a PreparedCreatureGroundOperations),
    Global(&'a PreparedGlobalGroundOperations),
}
impl<'a> ScopedGroundPreparedRef<'a> {
    pub(super) fn operations(self) -> &'a [SpellItemOperation] {
        match self {
            Self::Creature(p) => &p.operations,
            Self::Global(p) => &p.operations,
        }
    }
    pub(super) fn matches(self, cause: ScopedGroundCauseRef<'_>) -> bool {
        match (self, cause) {
            (Self::Creature(p), ScopedGroundCauseRef::Creature(c)) => {
                p.placement == c.placement
                    && p.generation == c.generation
                    && p.body == c.body
                    && p.cast == c.cast
            }
            (Self::Global(p), ScopedGroundCauseRef::Global(c)) => {
                p.scope_identity == c.identity.scope_identity
                    && p.body == c.body
                    && p.cast == c.cast
            }
            _ => false,
        }
    }
}
macro_rules! shared_digest {
    ($name:ident) => {
        pub(super) fn $name(self) -> [u8; 32] {
            match self {
                Self::Creature(c) => c.$name,
                Self::Global(c) => c.$name,
            }
        }
    };
}
impl<'a> ScopedGroundCauseRef<'a> {
    shared_digest!(content);
    shared_digest!(map);
    shared_digest!(frame);
    shared_digest!(body);
    shared_digest!(cast);
    pub(super) fn world(self) -> [u8; 16] {
        match self {
            Self::Creature(c) => c.world,
            Self::Global(c) => *c.identity.scope.world_id().as_bytes(),
        }
    }
    pub(super) fn scope(self) -> RuntimeScopeRefV1 {
        match self {
            Self::Creature(c) => c.scope,
            Self::Global(c) => c.identity.scope,
        }
    }
    pub(super) fn generation(self) -> u64 {
        match self {
            Self::Creature(c) => c.scope_generation,
            Self::Global(c) => c.identity.ownership_generation,
        }
    }
    pub(super) fn source_items(self) -> &'a [TypedDefinitionRef] {
        match self {
            Self::Creature(c) => &c.source_items,
            Self::Global(c) => &c.source_items,
        }
    }
    pub(super) fn definition(self) -> &'a TypedDefinitionRef {
        match self {
            Self::Creature(c) => &c.ability,
            Self::Global(c) => &c.definition,
        }
    }
    pub(super) fn occurrence(self) -> &'a str {
        match self {
            Self::Creature(c) => &c.occurrence,
            Self::Global(c) => &c.occurrence,
        }
    }
    pub(super) fn creature(self) -> Option<&'a TypedDefinitionRef> {
        match self {
            Self::Creature(c) => Some(&c.creature),
            Self::Global(_) => None,
        }
    }
    pub(super) fn actor_placement(self) -> Option<&'a [u8]> {
        match self {
            Self::Creature(c) => Some(c.placement.as_slice()),
            Self::Global(_) => None,
        }
    }
    pub(super) fn actor_generation(self) -> Option<String> {
        match self {
            Self::Creature(c) => Some(c.generation.to_string()),
            Self::Global(_) => None,
        }
    }
    pub(super) fn lock_identity(self) -> (&'a [u8; 16], u64) {
        match self {
            Self::Creature(c) => (&c.placement, c.generation),
            Self::Global(c) => (&c.identity.scope_identity, c.identity.ownership_generation),
        }
    }
    pub(super) fn cause_kind(self) -> i16 {
        match self {
            Self::Creature(c) => c.cause_kind(),
            Self::Global(_) => 4,
        }
    }
    pub(super) fn channel_bytes(self) -> Option<[u8; 16]> {
        match self.scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => Some(*channel_id.as_bytes()),
            _ => None,
        }
    }
    pub(super) fn instance_bytes(self) -> Option<[u8; 16]> {
        match self.scope() {
            RuntimeScopeRefV1::Instance { instance_id, .. } => Some(instance_id),
            _ => None,
        }
    }
    pub(super) fn scope_kind(self) -> i16 {
        if self.instance_bytes().is_some() {
            2
        } else {
            1
        }
    }
    pub(super) fn owner<'b>(self, authority: &'b SpellItemScopeAuthority) -> PhysicalItemOwner<'b> {
        match self {
            Self::Creature(_) => PhysicalItemOwner::Creature(authority),
            Self::Global(_) => PhysicalItemOwner::RegisteredGlobal(authority),
        }
    }
    pub(super) fn physical_cause(self, transaction: [u8; 16]) -> PhysicalItemCause<'a> {
        match self {
            Self::Creature(_) => PhysicalItemCause::Creature(transaction),
            Self::Global(_) => PhysicalItemCause::RegisteredGlobal(transaction),
        }
    }
    pub(super) fn valid_shape(self) -> bool {
        match self {
            Self::Creature(c) => {
                c.placement != [0; 16]
                    && c.generation != 0
                    && c.creature.family == "Creature"
                    && ((matches!(c.ability.family.as_str(), "Ability" | "Encounter")
                        && c.event_rule.is_none())
                        || (c.ability.family == "Creature"
                            && c.event_rule.is_some()
                            && c.ability == c.creature
                            && c.event_rule.is_some_and(|rule| {
                                rule.permits_source(&c.creature.production_key, c.scope_kind())
                            })
                            && c.source_items.len() == 1
                            && c.source_items[0].family == "Item"
                            && c.event_rule.is_some_and(|rule| {
                                rule.permits_item(&c.source_items[0].production_key)
                            })
                            && c.source_items[0].revision_ref == "definition-r1"))
                    && !c.creature.production_key.is_empty()
                    && !c.creature.revision_ref.is_empty()
            }
            Self::Global(c) => {
                c.identity.scope_identity
                    == match c.identity.scope {
                        RuntimeScopeRefV1::Channel { channel_id, .. } => *channel_id.as_bytes(),
                        RuntimeScopeRefV1::Instance { instance_id, .. } => instance_id,
                    }
                    && uuid_v7(&c.identity.scope_identity)
                    && uuid_v7(&c.identity.node)
                    && c.identity.ownership_generation != 0
                    && c.identity.node_registration_revision != 0
                    && c.identity.assignment_revision != 0
                    && c.identity.stamp.ordinal().get() != 0
                    && c.cast != [0; 32]
                    && c.client != [0; 32]
                    && c.activation != 0
                    && c.identity.stamp.generation().get() == c.identity.ownership_generation
                    && c.definition.family == "Encounter"
                    && c.definition.production_key == "oteryn:encounter.soul_war_taint_zones"
                    && c.definition.revision_ref == "definition-r1"
                    && c.source_items.len() == 1
                    && c.source_items[0].family == "Item"
                    && c.source_items[0].production_key == "oteryn:item.tibia.i33877"
                    && c.source_items[0].revision_ref == "definition-r1"
            }
        }
    }
    pub(super) fn intent(
        self,
        authority: &SpellItemScopeAuthority,
        operations: &[serde_json::Value],
    ) -> serde_json::Value {
        self.intent_fields(authority.world_bytes(), authority.generation(), operations)
    }
    fn intent_fields(
        self,
        world: [u8; 16],
        ownership_generation: u64,
        operations: &[serde_json::Value],
    ) -> serde_json::Value {
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|value| format!("{value:02x}"))
                .collect::<String>()
        };
        match self {
            Self::Creature(cause) => {
                let mut intent_value = serde_json::json!({"schema":"OTERYN_CREATURE_GROUND_CAUSE/v1","player":null,"cost":null,"caster_origin":null,"world":hex(&world),"channel":cause.channel_bytes().as_ref().map(|value|hex(value)),"ownership_generation":ownership_generation.to_string(),"creature":{"key":cause.creature.production_key,"revision":cause.creature.revision_ref,"placement":hex(&cause.placement),"generation":cause.generation.to_string()},"ability":reference(&cause.ability),"occurrence":cause.occurrence,"content":hex(&cause.content),"map":hex(&cause.map),"frame":hex(&cause.frame),"source_body":hex(&cause.body),"source_cast":hex(&cause.cast),"operations":operations});
                if cause.cause_kind() == 2 {
                    intent_value["schema"] = "OTERYN_ENCOUNTER_GROUND_CAUSE/v2".into();
                    intent_value["runtime_scope_kind"] = cause.scope_kind().into();
                    intent_value["instance"] = cause
                        .instance_bytes()
                        .as_ref()
                        .map(|value| hex(value))
                        .into();
                }
                if let Some(rule) = cause.event_rule {
                    intent_value["schema"] = "OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1".into();
                    intent_value["runtime_scope_kind"] = cause.scope_kind().into();
                    intent_value["instance"] = cause
                        .instance_bytes()
                        .as_ref()
                        .map(|value| hex(value))
                        .into();
                    intent_value["source_event"] =
                        serde_json::json!({"rule":rule.key(),"action_id":rule.action_id()});
                }
                intent_value
            }
            Self::Global(cause) => serde_json::json!({
                "schema":"OTERYN_REGISTERED_GLOBAL_GROUND_CAUSE/v1", "player":null,"cost":null,"caster_origin":null,"creature":null,
                "world":hex(&world),"channel":self.channel_bytes().as_ref().map(|value|hex(value)),"instance":self.instance_bytes().as_ref().map(|value|hex(value)),"runtime_scope_kind":self.scope_kind(),
                "ownership_generation":ownership_generation.to_string(),"ability":reference(&cause.definition),"occurrence":cause.occurrence,
                "global_event":{"name":"CreateGoshnarSpiteFire","action":cause.action.key(),"registration_scope":hex(&cause.identity.scope_identity),"node":hex(&cause.identity.node),"node_registration_revision":cause.identity.node_registration_revision.to_string(),"assignment_revision":cause.identity.assignment_revision.to_string(),"work_ordinal":cause.identity.stamp.ordinal().get().to_string()},
                "content":hex(&cause.content),"client_content":hex(&cause.client),"activation_sequence":cause.activation.to_string(),"map":hex(&cause.map),"frame":hex(&cause.frame),"source_body":hex(&cause.body),"source_cast":hex(&cause.cast),"operations":operations,
            }),
        }
    }
}

#[cfg(test)]
#[path = "scoped_global_tests.rs"]
mod tests;
