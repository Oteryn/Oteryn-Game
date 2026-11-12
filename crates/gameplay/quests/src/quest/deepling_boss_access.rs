//! Chosen native USE access retains the donor's personal Crystal 13 prerequisite.
use super::world_callbacks::{Callback, Position, Trigger};

/// Explicit Oteryn geometry; retained B3 source tiles are walkable and non-house.
/// The portable 1949 profile does not establish map custody: the current Channel
/// installs immovable Added Items and resolves their real handle/revision.
#[derive(Debug, Clone, Copy)]
pub struct NativePortal {
    pub key: &'static str,
    pub source_key: &'static str,
    pub position: Position,
    pub appearance: u16,
}

pub const BINDINGS: [NativePortal; 3] = [
    NativePortal {
        key: "oteryn:quest-callback/deeplings-native-boss-28574",
        source_key: "oteryn:quest-callback/deeplings-boss-28574",
        position: Position {
            x: 33637,
            y: 31376,
            floor: -9,
        },
        appearance: 1949,
    },
    NativePortal {
        key: "oteryn:quest-callback/deeplings-native-boss-28575",
        source_key: "oteryn:quest-callback/deeplings-boss-28575",
        position: Position {
            x: 33637,
            y: 31377,
            floor: -9,
        },
        appearance: 1949,
    },
    NativePortal {
        key: "oteryn:quest-callback/deeplings-native-boss-28576",
        source_key: "oteryn:quest-callback/deeplings-boss-28576",
        position: Position {
            x: 33637,
            y: 31379,
            floor: -9,
        },
        appearance: 1949,
    },
];

/// Added-entry matching joins its existing physical identity and revision owner.
pub fn matches(key: &str, position: Position, appearance: u16) -> bool {
    BINDINGS.iter().any(|binding| {
        binding.key == key && binding.position == position && binding.appearance == appearance
    })
}

/// Preserve the existing donor's personal Crystal 13 predicate and destination.
/// Only native geometry/USE activation is adapted. No UID is fabricated in B3.
pub fn callbacks(source: &[Callback]) -> Result<Vec<Callback>, String> {
    let mut result = Vec::with_capacity(BINDINGS.len());
    for binding in BINDINGS {
        let mut callback = source
            .iter()
            .find(|callback| callback.key == binding.source_key)
            .cloned()
            .ok_or_else(|| format!("missing Deepling boss source {}", binding.source_key))?;
        if callback.storage.len() != 1
            || callback.storage[0].minimum != 13
            || callback.storage[0].maximum != 13
            || callback.storage[0].set != 13
            || callback.teleport.is_none()
        {
            return Err(format!(
                "changed Deepling boss source {}",
                binding.source_key
            ));
        }
        callback.key = binding.key.into();
        callback.trigger = Trigger::Use;
        callback.selector.position = Some(binding.position);
        callback.selector.appearance = Some(binding.appearance);
        callback.selector.unique = None;
        callback.selector.action = None;
        callback.sources.push(concat!(
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:",
            "data-global/scripts/quests/deeplings_worldchange/movements_deepling_bosses.lua:2-4; ",
            "CHOSEN_OTERYN_NATIVE_USE_GEOMETRY_NOT_DONOR_POSITION_OR_UID",
        ).into());
        result.push(callback);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn worldchg_deepling_native_boss_access_retains_crystal_thirteen() {
        let source = super::super::world_callbacks::load().expect("source callbacks");
        let native = callbacks(&source).expect("native access");
        assert_eq!(native.len(), 3);
        assert!(
            callbacks(&[]).is_err(),
            "missing donor callbacks cannot fabricate access"
        );
        for (callback, binding) in native.iter().zip(BINDINGS) {
            let donor = source
                .iter()
                .find(|c| c.key == binding.source_key)
                .expect("donor");
            assert_eq!(callback.trigger, Trigger::Use);
            assert_eq!(
                super::super::world_callbacks::world_change_requirement(&callback.key),
                Some(("deeplings", Some(3)))
            );
            assert_eq!(callback.teleport, donor.teleport);
            assert_eq!(
                callback.storage, donor.storage,
                "existing personal Crystal 13 is unchanged"
            );
            assert_eq!(callback.storage[0].minimum, 13);
            assert_eq!(callback.storage[0].maximum, 13);
            assert_eq!(callback.storage[0].set, 13);
            assert!(callback.selector.unique.is_none() && callback.selector.action.is_none());
            assert!(matches(&callback.key, binding.position, 1949));
            assert!(!matches(&callback.key, binding.position, 1948));
            assert!(!matches(
                &callback.key,
                Position {
                    x: binding.position.x + 1,
                    ..binding.position
                },
                1949
            ));
        }
        assert_eq!(
            BINDINGS.map(|b| b.position),
            [
                Position {
                    x: 33637,
                    y: 31376,
                    floor: -9
                },
                Position {
                    x: 33637,
                    y: 31377,
                    floor: -9
                },
                Position {
                    x: 33637,
                    y: 31379,
                    floor: -9
                }
            ]
        );
    }
}
