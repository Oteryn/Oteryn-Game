//! Source NPC cues reuse the qualified presentation bindings and existing client protocol.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    #[default]
    Player,
    Npc,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub effect: String,
    #[serde(default)]
    pub target: Target,
}

impl Cue {
    /// Resolve a source constant, never a client-supplied numeric asset identity.
    pub fn effect_id(&self) -> Option<u16> {
        if !self.effect.starts_with("CONST_ME_") {
            return None;
        }
        static EFFECTS: OnceLock<Option<BTreeMap<String, u16>>> = OnceLock::new();
        EFFECTS
            .get_or_init(|| {
                #[derive(Deserialize)]
                struct Bindings {
                    schema: String,
                    records: Vec<Binding>,
                }
                #[derive(Deserialize)]
                struct Binding {
                    kind: String,
                    source_member: Option<String>,
                    client_id: Option<u16>,
                }
                let document: Bindings = serde_json::from_str(include_str!(
                    "../../../../../content/presentations/bindings/client-ids.json"
                ))
                .ok()?;
                if document.schema != "OTERYN_PRESENTATION_CLIENT_IDS/v1" {
                    return None;
                }
                let mut effects = BTreeMap::new();
                for row in document.records {
                    if row.kind != "effect" {
                        continue;
                    }
                    let (Some(name), Some(id @ 1..)) = (row.source_member, row.client_id) else {
                        continue;
                    };
                    if effects
                        .insert(name, id)
                        .is_some_and(|previous| previous != id)
                    {
                        return None;
                    }
                }
                Some(effects)
            })
            .as_ref()?
            .get(&self.effect)
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_magic_cues_resolve_only_qualified_effect_bindings() {
        for (effect, id) in [
            ("CONST_ME_MAGIC_BLUE", 13),
            ("CONST_ME_YELLOWENERGY", 49),
            ("CONST_ME_HOLYAREA", 50),
        ] {
            assert_eq!(
                Cue {
                    effect: effect.into(),
                    target: Target::Player
                }
                .effect_id(),
                Some(id)
            );
        }
        for effect in [
            "50",
            "CONST_ME_UNKNOWN_QNPCFX",
            "canary.appearance:effect/holyarea",
        ] {
            assert_eq!(
                Cue {
                    effect: effect.into(),
                    target: Target::Player
                }
                .effect_id(),
                None
            );
        }
    }
}
