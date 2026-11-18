//! Retained Crystal 9f5a72c bindings for immutable historical tool receipts.
//! Fresh painting belongs to `ice_islands_seals` and its three-distinct-target rule.
use super::world_callbacks::{Callback, Position};
pub const PREFIX: &str = "oteryn:quest-callback/engitem-ice-seal-";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
pub const PAINT: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/paint_seal";
pub const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission04";
pub const SEALS: [Position; 3] = [
    Position {
        x: 32380,
        y: 31157,
        floor: -7,
    },
    Position {
        x: 32380,
        y: 31158,
        floor: -7,
    },
    Position {
        x: 32381,
        y: 31158,
        floor: -7,
    },
];

/// Keep old receipt identities without exposing their repeatable counter as a
/// fresh alternative to the current distinct-target owner.
pub fn historical_only(key: &str) -> bool {
    key.strip_prefix(PREFIX).is_some_and(|suffix| {
        matches!(
            suffix,
            "0-0" | "0-1" | "0-2" | "1-0" | "1-1" | "1-2" | "2-0" | "2-1" | "2-2"
        )
    })
}

pub fn callbacks() -> Result<Vec<Callback>, String> {
    let mut result = Vec::new();
    for (site, position) in SEALS.into_iter().enumerate() {
        for phase in 0..3 {
            let last = phase == 2;
            let value = serde_json::json!({"schema":1,"callbacks":[{
                "key":format!("{PREFIX}{site}-{phase}"),"quest":"oteryn:quest.the_ice_islands_quest","trigger":"use",
                "selector":{"native":{"position":position,"ordinal":2,"appearance":7178},"position":position,"unique":null,"action":null,"appearance":7178},
                "requires_world_admission":true,"lever":null,"reversible":false,"passages":[],"teleport":null,
                "storage":[
                    {"track":QUESTLINE,"minimum":8,"maximum":8,"set":if last {9}else{8},"preserve":!last},
                    {"track":MISSION,"minimum":1,"maximum":1,"set":if last {2}else{1},"preserve":!last},
                    {"track":PAINT,"minimum":phase-1,"maximum":phase-1,"set":phase}
                ],
                "offerings":[],"item_on_target":{"item":"oteryn:item.tibia.i7253","count":1,"replacement":null,"target_replacement":null,"reusable":!last,"selected_instance":true},
                "effects":{"transform":7252,"reset_after_ms":30000,"player_speech":"You painted a baby seal.","magic":{"position":position,"effect":9}},
                "return_on_failure":false,"failure_message":null,"success_message":null,
                "sources":["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:9-31",
                    "canary:04b83b512114bfd888000d6e1433ed8ecaec7c5b:src/utils/utils_definitions.hpp:CONST_ME_GREEN_RINGS=9"]
            }]});
            result.extend(super::world_callbacks::parse(&value.to_string())?);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_seals_keep_negative_initial_count_thirty_second_reset_and_final_cost()
    -> Result<(), String> {
        let all = callbacks()?;
        assert_eq!(all.len(), 9);
        assert!(all.iter().all(|callback| historical_only(&callback.key)));
        for key in [
            "oteryn:quest-callback/engitem-ice-seal-0-3",
            "oteryn:quest-callback/engitem-ice-seal-3-0",
            "oteryn:quest-callback/ice-islands/paint-seal-120",
            "oteryn:quest-callback/srv7-reliable-ram",
        ] {
            assert!(!historical_only(key));
        }
        for group in all.as_chunks::<3>().0 {
            assert_eq!(group[0].storage[2].minimum, -1);
            for (phase, c) in group.iter().enumerate() {
                assert_eq!(c.effects.reset_after_ms, Some(30000));
                assert_eq!(c.effects.transform, Some(7252));
                assert_eq!(
                    c.item_on_target.as_ref().map(|i| i.reusable),
                    Some(phase < 2)
                );
                assert!(super::super::source_world_objects::source_is_ready(c, 7178));
                assert!(!super::super::source_world_objects::source_is_ready(
                    c, 7252
                ));
            }
            assert_eq!(
                (
                    group[2].storage[0].set,
                    group[2].storage[1].set,
                    group[2].storage[2].set
                ),
                (9, 2, 2)
            );
        }
        Ok(())
    }
}
