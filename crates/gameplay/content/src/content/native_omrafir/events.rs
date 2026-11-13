//! Closed source onThink and retained birth evidence; no Ability slot is invented.
use super::super::native_monster_callbacks::QualifiedNativeCreatureEventSpawn;
use super::*;
#[derive(Debug, Clone)]
pub struct QualifiedOmrafirEvents {
    content: [u8; 32],
    parent: ProjectV2DefinitionRef,
    hellfire: ProjectV2DefinitionRef,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmrafirEventBirth {
    Hellfire,
    NormalReform,
}
impl QualifiedOmrafirEvents {
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
    pub fn parent(&self) -> &ProjectV2DefinitionRef {
        &self.parent
    }
    pub fn hellfire(&self) -> &ProjectV2DefinitionRef {
        &self.hellfire
    }
    pub fn spawn_source(&self, kind: OmrafirEventBirth) -> QualifiedNativeCreatureEventSpawn {
        super::super::native_monster_callbacks::omrafir_event_spawn(self, kind)
    }
    pub fn terminal_source_outcome(&self) -> &'static str {
        "unknown-monster-nil-then-lua-error"
    }
}
impl NativeGameplayState {
    pub fn omrafir_events_source(
        &self,
        creature: &ProjectV2DefinitionRef,
    ) -> Option<QualifiedOmrafirEvents> {
        qualify_events_document(self, creature, self.creature_profiles())
    }
}
pub(super) fn qualify_events_document(
    native: &NativeGameplayState,
    creature: &ProjectV2DefinitionRef,
    document: &super::super::native_gameplay::CreatureProfilesDocument,
) -> Option<QualifiedOmrafirEvents> {
    native.omrafir_source_attacks(creature)?;
    let witness = source_witness()?;
    let children = witness.event_children.as_ref()?;
    let [child] = children.as_slice() else {
        return None;
    };
    if witness.creature_events.as_ref()? != &serde_json::json!({
            "callback":{"path":"data-otservbr-global/scripts/creaturescripts/monster/omrafir_explode.lua",
                "sha256":"bad7511af294bebc5a9d94345252d0ba03784172a73463d772be6350a467cbcf"},
            "hp_at_most_ppm":500000,"terminal_hp_at_most_ppm":30000,"summons_below":4,
            "boom_delay_ms":10,"reform_delay_ms":26764,
            "storage":{"key":112416,"initial":-1,"normal_reform_below":2},
            "summon":{"name":"Hellfire Fighter","amount":9,"center":[33589,32379,12],
                "offset_x":[-8,8],"offset_y":[-9,9],"extended":true,"forced":true},
            "reform":{"name":"Omrafir","position":[33586,32379,12],"extended":false,"forced":true},
            "terminal_reform":{"name":"Omrafir2","position":[33586,32379,12],"extended":false,"forced":true,
                "result":"unknown-monster-nil-then-lua-error"}})
            || child.source_binding.target != child.native.profile.target
            || child.source_binding.source_key!="oteryn:source.canary"
            || child.source_binding.source_revision!="47dfd51f45280a59a1d3e50ba7edd573d7234446"
            || child.source_binding.identity_namespace!="canary/monster-file"
            || child.source_binding.external_id!="demons/hellfire_fighter"
            || child.source_binding.disposition!=super::super::ProjectV2SourceIdentityDisposition::Exact
            || document.records.iter().filter(|r|r.profile.target==child.native.profile.target).count()!=1
            || document.records.iter().find(|r|r.profile.target==child.native.profile.target)!=Some(&child.native)
            || document.records.iter().any(|r|matches!(&r.profile.data,
                super::super::ProjectV2AuthoringProfileData::Creature(c) if c.details.as_ref().is_some_and(|d|d.display_name.eq_ignore_ascii_case("Omrafir2"))))
        {return None;}
    Some(QualifiedOmrafirEvents {
        content: native.source_digest(),
        parent: creature.clone(),
        hellfire: child.native.profile.target.clone(),
    })
}
