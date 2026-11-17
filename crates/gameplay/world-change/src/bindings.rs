//! Executable native bindings, distinct from unresolved selection-source candidates.
//! These name existing physical owners; they never invent geometry or admit held content.
use crate::daily::{DailyWorldChanges, FURY_LAYERS, NIGHTMARE_LAYERS};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Binding {
    Layer {
        key: String,
        exit_only: bool,
    },
    Npc {
        key: String,
        position: (u16, u16, i8),
    },
    Item {
        appearance: u16,
        position: (u16, u16, i8),
    },
    SpawnPolicy {
        key: String,
        variant: String,
    },
    Raid {
        key: String,
        requirement: String,
    },
    CounterPolicy {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeBindings {
    schema: u8,
    epoch: u64,
    selection: crate::decision::NativeDecision,
    fire_from_the_earth: bool,
    stampede: bool,
    world_counters: bool,
    bindings: Vec<Binding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNativeBinding;

mod active_layer;

impl NativeBindings {
    /// Compile only identities already implemented by map, spawn, NPC and raid owners.
    /// The durable owner binds the resulting bytes to the World and immutable reset day.
    pub fn compile(day: &DailyWorldChanges) -> Result<Self, InvalidNativeBinding> {
        if !FURY_LAYERS.contains(&day.fury_exit)
            || !NIGHTMARE_LAYERS.contains(&day.nightmare_isles)
            || day.fury_gates.is_some_and(|layer| layer != day.fury_exit)
        {
            return Err(InvalidNativeBinding);
        }
        let mut result = Self {
            schema: 1,
            epoch: day.reset_epoch,
            selection: crate::decision::NativeDecision::capture(day),
            fire_from_the_earth: day.fire_from_the_earth,
            stampede: day.stampede,
            world_counters: day.world_counters,
            bindings: Vec::new(),
        };
        // Boot without a committed save has no physical activation authority.
        if day.reset_epoch == 0 {
            return Ok(result);
        }
        for key in day.layers() {
            result.bindings.push(Binding::Layer {
                key: key.into(),
                exit_only: false,
            });
        }
        for key in day.exit_layers() {
            result.bindings.push(Binding::Layer {
                key: key.into(),
                exit_only: true,
            });
        }
        for (key, position) in day.npcs() {
            result.bindings.push(Binding::Npc {
                key: key.into(),
                position,
            });
        }
        if let Some(town) = day.oriental_trader {
            result.bindings.push(Binding::Layer {
                key: town.layer().into(),
                exit_only: false,
            });
            result.bindings.push(Binding::Npc {
                key: "oteryn:npc.yasir".into(),
                position: town.yasir_position(),
            });
        }
        for (appearance, position) in day.items() {
            result.bindings.push(Binding::Item {
                appearance,
                position,
            });
        }
        result.bindings.push(Binding::SpawnPolicy {
            key: "grimvale".into(),
            variant: if day.grimvale_full_moon {
                "full_moon"
            } else {
                "ordinary"
            }
            .into(),
        });
        if day.their_masters_voice {
            result.bindings.push(Binding::SpawnPolicy {
                key: "their_masters_voice".into(),
                variant: "fungus".into(),
            });
        }
        if day.stampede {
            result.bindings.push(Binding::SpawnPolicy {
                key: "stampede".into(),
                variant: "active".into(),
            });
        }
        for kind in crate::counter::CounterKind::ALL {
            if crate::owner::counter_admitted(kind) {
                result.bindings.push(Binding::CounterPolicy {
                    key: kind.key().into(),
                });
            }
        }
        for (key, requirement) in raid_requirements()? {
            result.bindings.push(Binding::Raid { key, requirement });
        }
        Ok(result)
    }

    pub fn encoded(&self) -> Result<Vec<u8>, InvalidNativeBinding> {
        serde_json::to_vec(self).map_err(|_| InvalidNativeBinding)
    }
}

/// Shared preconditions consumed by the production raid scheduler and native binding compiler.
pub fn raid_requirements()
-> Result<std::collections::BTreeMap<String, String>, InvalidNativeBinding> {
    let catalogue: serde_json::Value =
        serde_json::from_str(crate::owner::CATALOGUE_JSON).map_err(|_| InvalidNativeBinding)?;
    let entries = catalogue["entries"]
        .as_array()
        .filter(|entries| entries.len() <= 256)
        .ok_or(InvalidNativeBinding)?;
    let mut requirements = std::collections::BTreeMap::new();
    for entry in entries {
        let Some(binding) = entry
            .get("binding")
            .filter(|binding| binding["kind"] == "raid")
        else {
            continue;
        };
        let key = entry["key"]
            .as_str()
            .filter(|key| !key.is_empty() && key.len() <= 64)
            .ok_or(InvalidNativeBinding)?;
        let raid = binding["raid_key"]
            .as_str()
            .filter(|raid| !raid.is_empty() && raid.len() <= 512)
            .ok_or(InvalidNativeBinding)?;
        let requirement = if let Some(stage) = binding.get("stage") {
            let stage = stage
                .as_u64()
                .filter(|stage| (1..=5).contains(stage))
                .ok_or(InvalidNativeBinding)?;
            if crate::counter::CounterKind::from_key(key).is_none() {
                return Err(InvalidNativeBinding);
            }
            format!("{key}@{stage}")
        } else {
            key.to_owned()
        };
        if requirements.insert(raid.to_owned(), requirement).is_some() {
            return Err(InvalidNativeBinding);
        }
    }
    Ok(requirements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bindings_reject_cross_family_layers_and_mismatched_returns() -> Result<(), String> {
        let day = DailyWorldChanges::roll(1, 6, 13, |_| 0);
        for bad in [
            DailyWorldChanges {
                fury_exit: NIGHTMARE_LAYERS[0],
                ..day
            },
            DailyWorldChanges {
                nightmare_isles: FURY_LAYERS[0],
                ..day
            },
            DailyWorldChanges {
                fury_gates: Some(FURY_LAYERS[1]),
                ..day
            },
        ] {
            assert_eq!(NativeBindings::compile(&bad), Err(InvalidNativeBinding));
        }
        let compiled = NativeBindings::compile(&day).map_err(|e| format!("{e:?}"))?;
        assert!(
            compiled
                .bindings
                .iter()
                .any(|b| matches!(b, Binding::Npc { key, .. } if key == "oteryn:npc.yasir"))
        );
        assert!(compiled.bindings.iter().any(|b| matches!(b, Binding::SpawnPolicy { key, variant } if key == "grimvale" && variant == "full_moon")));
        assert!(compiled.bindings.iter().any(
            |b| matches!(b, Binding::Raid { requirement, .. } if requirement == "fury_gates")
        ));
        Ok(())
    }

    #[test]
    fn uncommitted_epoch_has_no_native_bindings() -> Result<(), String> {
        let day = DailyWorldChanges::roll(0, 6, 13, |_| 0);
        assert!(
            NativeBindings::compile(&day)
                .map_err(|e| format!("{e:?}"))?
                .bindings
                .is_empty()
        );
        for key in ["grimvale", "fury_gates", "hive_born", "hive_born@1"] {
            assert!(!crate::owner::is_active(&day, key));
        }
        Ok(())
    }
}
