//! Durable, content-qualified terrain-tool transformations.
//!
//! The network owner resolves the carried tool and map placement before calling this module.
//! This module deliberately has no numeric fallback: only catalogue entries admitted by the
//! active content generation can produce a write plan.

use std::time::Duration;

/// A tool identity from the pinned item catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTool {
    ElementalSoil(u16),
    Shovel,
    Pick,
    Machete,
    Scythe,
    Sickle,
    Crowbar,
    SlimeGobbler,
}

impl TerrainTool {
    /// Resolve the Crystal-compatible tool from its qualified Oteryn production key.
    pub fn from_production_key(key: &str) -> Option<Self> {
        // Crystal 201189f8 data-global/scripts/lib/register_actions.lua:884-943 and
        // :1035-1045; data/scripts/actions/tools/sickle.lua:4-7.
        const SCYTHES: [&str; 2] = ["oteryn:item.tibia.i3453", "oteryn:item.tibia.i9596"];
        const SICKLES: [&str; 1] = ["oteryn:item.tibia.i3293"];
        const CROWBARS: [&str; 2] = ["oteryn:item.tibia.i3304", "oteryn:item.tibia.i9598"];
        if SCYTHES.contains(&key) {
            return Some(Self::Scythe);
        }
        if SICKLES.contains(&key) {
            return Some(Self::Sickle);
        }
        if CROWBARS.contains(&key) {
            return Some(Self::Crowbar);
        }
        Some(match key {
            "oteryn:item.tibia.i940" => Self::ElementalSoil(940),
            "oteryn:item.tibia.i941" => Self::ElementalSoil(941),
            "oteryn:item.tibia.i944" => Self::ElementalSoil(944),
            "oteryn:item.tibia.i3457" | "oteryn:item.tibia.i5710" => Self::Shovel,
            "oteryn:item.tibia.i3456" => Self::Pick,
            // Crystal register_actions.lua:864-885 routes each Secret Agent multi-tool
            // through the matching ordinary tool action.
            "oteryn:item.tibia.i3308" | "oteryn:item.tibia.i3330" | "oteryn:item.tibia.i9594" => {
                Self::Machete
            }
            "oteryn:item.tibia.i12077" => Self::SlimeGobbler,
            _ => return None,
        })
    }
}

/// One admitted state in a transform/regrowth chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecayStep {
    pub state: u32,
    pub after: Duration,
}

/// Qualified mapping. A nonempty `yield_item` requires the owner to mint atomically with the
/// first state change or refuse the request. Each decay step requires a durable deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainToolBinding {
    pub tool: TerrainTool,
    pub target: u32,
    pub required_action: Option<u16>,
    pub transformed: u32,
    pub yield_item: Option<u32>,
    pub decay: Vec<DecayStep>,
}

/// A fully resolved write request for the durable owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainTransformPlan<'a> {
    pub binding: &'a TerrainToolBinding,
    pub first_deadline_unix_ms: Option<i64>,
}

/// Fail-closed item-use dispatch. The caller must additionally prove current carried custody and
/// target revision; this function proves the tool/target pair and computes the durable deadline.
pub fn dispatch<'a>(
    catalogue: &'a [TerrainToolBinding],
    tool_key: &str,
    target_state: u32,
    target_action: Option<u16>,
    committed_at_unix_ms: i64,
) -> Option<TerrainTransformPlan<'a>> {
    let tool = TerrainTool::from_production_key(tool_key)?;
    let binding = catalogue.iter().find(|binding| {
        binding.tool == tool
            && binding.target == target_state
            && binding.required_action == target_action
    })?;
    let first_deadline_unix_ms = binding.decay.first().and_then(|step| {
        i64::try_from(step.after.as_millis())
            .ok()
            .and_then(|delay| committed_at_unix_ms.checked_add(delay))
    });
    if !binding.decay.is_empty() && first_deadline_unix_ms.is_none() {
        return None;
    }
    Some(TerrainTransformPlan {
        binding,
        first_deadline_unix_ms,
    })
}

/// Bounded mappings. Digging and jungle grass are backed by Canary register_actions.lua at
/// 47dfd51f and the pinned items.xml. Harvest plans are staged: the runtime must refuse them
/// until their yield has an audited mint owner. The sickle plan is Crystal 201189f8
/// sickle.lua:12-15 (5463 to 5462, 5466 created on the target tile) with items.xml:14996-15027
/// regrowth. Crystal data-crystal/scripts/lib/register_actions.lua:448-452 delegates
/// crowbar cask row at register_actions.lua:946-954 transforms for three minutes.
pub fn crystal_core_bindings() -> Vec<TerrainToolBinding> {
    let mut bindings = vec![
        TerrainToolBinding {
            tool: TerrainTool::Pick,
            target: 867,
            required_action: None,
            transformed: 868,
            yield_item: None,
            decay: vec![DecayStep {
                state: 867,
                after: Duration::from_secs(600),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Machete,
            target: 3696,
            required_action: None,
            transformed: 3695,
            yield_item: None,
            decay: vec![DecayStep {
                state: 3696,
                after: Duration::from_secs(300),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Machete,
            target: 3702,
            required_action: None,
            transformed: 3701,
            yield_item: None,
            decay: vec![DecayStep {
                state: 3702,
                after: Duration::from_secs(300),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Machete,
            target: 17153,
            required_action: None,
            transformed: 17151,
            yield_item: None,
            decay: vec![DecayStep {
                state: 17153,
                after: Duration::from_secs(300),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Shovel,
            target: 867,
            required_action: None,
            transformed: 868,
            yield_item: None,
            decay: vec![DecayStep {
                state: 867,
                after: Duration::from_secs(600),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Pick,
            target: 354,
            required_action: Some(101),
            transformed: 394,
            yield_item: None,
            decay: vec![DecayStep {
                state: 354,
                after: Duration::from_secs(300),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Pick,
            target: 355,
            required_action: Some(101),
            transformed: 394,
            yield_item: None,
            decay: vec![DecayStep {
                state: 354,
                after: Duration::from_secs(300),
            }],
        },
        TerrainToolBinding {
            tool: TerrainTool::Scythe,
            target: 3653,
            required_action: None,
            transformed: 3651,
            yield_item: Some(3605),
            decay: vec![
                DecayStep {
                    state: 3652,
                    after: Duration::from_secs(240),
                },
                DecayStep {
                    state: 3653,
                    after: Duration::from_secs(240),
                },
            ],
        },
        TerrainToolBinding {
            tool: TerrainTool::Scythe,
            target: 5464,
            required_action: None,
            transformed: 5463,
            yield_item: Some(5466),
            decay: vec![
                DecayStep {
                    state: 5470,
                    after: Duration::from_secs(240),
                },
                DecayStep {
                    state: 5465,
                    after: Duration::from_secs(240),
                },
            ],
        },
        TerrainToolBinding {
            tool: TerrainTool::Sickle,
            target: 5463,
            required_action: None,
            transformed: 5462,
            yield_item: Some(5466),
            decay: vec![
                DecayStep {
                    state: 5470,
                    after: Duration::from_secs(240),
                },
                DecayStep {
                    state: 5465,
                    after: Duration::from_secs(240),
                },
            ],
        },
        TerrainToolBinding {
            tool: TerrainTool::Crowbar,
            target: 4848,
            required_action: Some(40041),
            transformed: 3134,
            yield_item: None,
            decay: vec![DecayStep {
                state: 4848,
                after: Duration::from_secs(180),
            }],
        },
    ];
    // M08B: pick holes without an items.xml duration/decayTo stay open until reset.
    // Crystal data/items/items.xml:18875-18881 (7236 has none; 7237 -> 7200 in 900 s)
    // and :42477-42479 (21342 has no decay). Do not apply spent fishing-hole decay here.
    bindings.extend([(593, 594, 60), (606, 615, 300), (608, 609, 300)].map(
        |(target, transformed, secs)| TerrainToolBinding {
            tool: TerrainTool::Pick,
            target,
            required_action: None,
            transformed,
            yield_item: None,
            decay: vec![DecayStep {
                state: target,
                after: Duration::from_secs(secs),
            }],
        },
    ));
    // Crystal 201189f8 data-global/scripts/lib/register_actions.lua:629,637.
    const QUEST_PICK_ROWS: [(u32, u32); 2] = [(7200, 7236), (21341, 21342)];
    bindings.extend(
        QUEST_PICK_ROWS.map(|(target, transformed)| TerrainToolBinding {
            tool: TerrainTool::Pick,
            target,
            required_action: None,
            transformed,
            yield_item: None,
            decay: Vec::new(),
        }),
    );
    // Global's loose stone piles all open to their immediately following appearance. Keep this
    // family data-driven so the production USE_WITH path applies the same custody, CAS, receipt,
    // and deadline recovery as the large-hole mapping above.
    bindings.extend([593, 606, 608, 21341].map(|target| TerrainToolBinding {
        tool: TerrainTool::Shovel,
        target,
        required_action: None,
        transformed: target + 1,
        yield_item: None,
        decay: vec![DecayStep {
            state: target,
            after: Duration::from_secs(600),
        }],
    }));
    // Crystal their_masters_voice.lua:43,111-127. The encounter's one-hour reset is
    // shared, so these individual mutations must not invent independent regrowth clocks.
    bindings.extend((12059..=12063).map(|target| TerrainToolBinding {
        tool: TerrainTool::SlimeGobbler,
        target,
        required_action: None,
        transformed: 12065,
        yield_item: None,
        decay: Vec::new(),
    }));
    bindings.extend(
        oteryn_gameplay_items::elemental_soil::CHARGES.map(|(soil, before, after)| {
            TerrainToolBinding {
                tool: TerrainTool::ElementalSoil(soil),
                target: u32::from(before),
                required_action: None,
                transformed: u32::from(after),
                yield_item: None,
                decay: vec![DecayStep {
                    state: u32::from(before),
                    after: Duration::from_secs(120),
                }],
            }
        }),
    );
    bindings
}

/// Crystal their_masters_voice.lua:9-41,43-49,59-83 (SHA256
/// 9024836cf5c3cf570067c58383e9c4bd548b409d8650029d02ed91e36b5d746a).
/// These are source home positions, in carrier floor coordinates. The existing
/// World spawn projection supplies deterministic World/encounter/wave draws.
pub const VOICE_SERVANT_HOMES: [(i32, i32, i16); 25] = [
    (33313, 31852, 9),
    (33313, 31881, 9),
    (33328, 31860, 9),
    (33328, 31873, 9),
    (33328, 31885, 9),
    (33308, 31873, 9),
    (33320, 31873, 9),
    (33335, 31873, 9),
    (33360, 31873, 9),
    (33336, 31914, 9),
    (33343, 31914, 9),
    (33353, 31914, 9),
    (33361, 31914, 9),
    (33345, 31900, 9),
    (33352, 31900, 9),
    (33355, 31854, 9),
    (33355, 31885, 9),
    (33345, 31864, 9),
    (33345, 31881, 9),
    (33309, 31867, 9),
    (33317, 31879, 9),
    (33311, 31854, 9),
    (33334, 31889, 9),
    (33340, 31890, 9),
    (33347, 31889, 9),
];
pub const VOICE_MAGE_HOMES: [(i32, i32, i16); 3] =
    [(33328, 31859, 9), (33367, 31873, 9), (33349, 31899, 9)];

/// Resolve an owned native home slot, never the creature's roaming death position.
pub fn voice_servant_ordinal(home: (i32, i32, i16)) -> Option<u8> {
    VOICE_SERVANT_HOMES
        .iter()
        .position(|&p| p == home)
        .and_then(|ordinal| u8::try_from(ordinal).ok())
}

pub type VoiceSpawn = (&'static str, (i32, i32, i16));

/// Source one independent 1..100 draw per slot: 10% diamond, 30% golden, 60% iron.
/// Slot and wave generation remain attached to the resulting native spawn cohort.
pub fn voice_servants(mut draw: impl FnMut(u8) -> u64) -> [VoiceSpawn; 25] {
    std::array::from_fn(|ordinal| {
        let roll = ((u128::from(draw(ordinal as u8)) * 100) >> 64) as u8;
        let creature = match roll {
            0..=9 => "oteryn:creature.diamond_servant",
            10..=39 => "oteryn:creature.golden_servant",
            _ => "oteryn:creature.iron_servant",
        };
        (creature, VOICE_SERVANT_HOMES[ordinal])
    })
}

/// This quest's three source choices exclude the independent legacy startup spawn.
pub fn voice_mage(draw: u64) -> VoiceSpawn {
    let ordinal = ((u128::from(draw) * 3) >> 64) as usize;
    ("oteryn:creature.mad_mage", VOICE_MAGE_HOMES[ordinal])
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn tool_dispatch_maps_qualified_pairs_and_refuses_wrong_targets() {
        let mut catalogue = crystal_core_bindings();
        catalogue.push(TerrainToolBinding {
            tool: TerrainTool::Machete,
            target: 9001,
            required_action: None,
            transformed: 9002,
            yield_item: None,
            decay: vec![DecayStep {
                state: 9001,
                after: Duration::from_secs(60),
            }],
        });
        catalogue.push(TerrainToolBinding {
            tool: TerrainTool::Crowbar,
            target: 9010,
            required_action: None,
            transformed: 9011,
            yield_item: None,
            decay: Vec::new(),
        });
        // Fixture-only sickle mapping keeps generic family coverage; no real sickle target
        // is admitted without source evidence.
        catalogue.push(TerrainToolBinding {
            tool: TerrainTool::Sickle,
            target: 9020,
            required_action: None,
            transformed: 9021,
            yield_item: None,
            decay: Vec::new(),
        });
        let cases = [
            ("oteryn:item.tibia.i3457", 593, 594),
            ("oteryn:item.tibia.i3457", 606, 607),
            ("oteryn:item.tibia.i5710", 608, 609),
            ("oteryn:item.tibia.i3457", 867, 868),
            ("oteryn:item.tibia.i5710", 867, 868),
            ("oteryn:item.tibia.i3456", 354, 394),
            ("oteryn:item.tibia.i3456", 867, 868),
            ("oteryn:item.tibia.i3308", 3696, 3695),
            ("oteryn:item.tibia.i3308", 3702, 3701),
            ("oteryn:item.tibia.i3308", 17153, 17151),
            ("oteryn:item.tibia.i3330", 17153, 17151),
            ("oteryn:item.tibia.i9594", 17153, 17151),
            ("oteryn:item.tibia.i3308", 9001, 9002),
            ("oteryn:item.tibia.i3453", 3653, 3651),
            ("oteryn:item.tibia.i9596", 3653, 3651),
            ("oteryn:item.tibia.i3453", 5464, 5463),
            ("oteryn:item.tibia.i9596", 5464, 5463),
            ("oteryn:item.tibia.i3293", 5463, 5462),
            ("oteryn:item.tibia.i3456", 593, 594),
            ("oteryn:item.tibia.i3456", 606, 615),
            ("oteryn:item.tibia.i3456", 608, 609),
            ("oteryn:item.tibia.i3293", 9020, 9021),
            ("oteryn:item.tibia.i3304", 9010, 9011),
            ("oteryn:item.tibia.i9598", 9010, 9011),
            ("oteryn:item.tibia.i3457", 21341, 21342),
        ];
        for (tool, target, transformed) in cases {
            assert_eq!(
                dispatch(
                    &catalogue,
                    tool,
                    target,
                    (target == 354).then_some(101),
                    1_000
                )
                .expect("admitted tool binding")
                .binding
                .transformed,
                transformed
            );
            assert!(dispatch(&catalogue, tool, target + 10_000, None, 1_000).is_none());
        }
        assert!(dispatch(&catalogue, "oteryn:item.tibia.i9999", 867, None, 1_000).is_none());
    }

    #[test]
    fn fragile_ground_requires_the_source_action_and_deadline_overflow_fails_closed() {
        let catalogue = crystal_core_bindings();
        for state in [354, 355] {
            assert!(dispatch(&catalogue, "oteryn:item.tibia.i3456", state, None, 0).is_none());
            assert!(dispatch(&catalogue, "oteryn:item.tibia.i3456", state, Some(100), 0).is_none());
            assert!(dispatch(&catalogue, "oteryn:item.tibia.i3456", state, Some(101), 0).is_some());
        }
        assert!(dispatch(&catalogue, "oteryn:item.tibia.i3457", 867, None, i64::MAX).is_none());
    }

    #[test]
    fn schedules_absolute_decay_and_preserves_harvest_yield() {
        let catalogue = crystal_core_bindings();
        let wheat = dispatch(&catalogue, "oteryn:item.tibia.i3453", 3653, None, 10_000)
            .expect("admitted tool binding");
        assert_eq!(wheat.binding.yield_item, Some(3605));
        assert_eq!(wheat.first_deadline_unix_ms, Some(250_000));
        assert_eq!(
            wheat
                .binding
                .decay
                .iter()
                .map(|step| step.state)
                .collect::<Vec<_>>(),
            [3652, 3653]
        );
    }

    #[test]
    fn m08_rows_preserve_harvest_yields_multitools_and_crowbar_cask_regrowth() {
        let catalogue = crystal_core_bindings();
        for (target, transformed, deadline) in
            [(593, 594, 70_000), (606, 615, 310_000), (608, 609, 310_000)]
        {
            let plan = dispatch(&catalogue, "oteryn:item.tibia.i3456", target, None, 10_000)
                .expect("terrain pick binding");
            assert_eq!(plan.binding.transformed, transformed);
            assert_eq!(plan.first_deadline_unix_ms, Some(deadline));
            assert_eq!(plan.binding.decay.len(), 1);
            assert_eq!(plan.binding.decay[0].state, target);
        }
        for (target, transformed) in [(7200, 7236), (21341, 21342)] {
            let plan = dispatch(&catalogue, "oteryn:item.tibia.i3456", target, None, 0)
                .expect("terrain pick binding");
            assert_eq!(plan.binding.transformed, transformed);
            assert!(plan.binding.decay.is_empty());
            assert_eq!(plan.first_deadline_unix_ms, None);
        }
        let sickle =
            dispatch(&catalogue, "oteryn:item.tibia.i3293", 5463, None, 0).expect("sickle binding");
        assert_eq!(sickle.binding.yield_item, Some(5466));
        for tool in ["oteryn:item.tibia.i3304", "oteryn:item.tibia.i9598"] {
            let cask = dispatch(&catalogue, tool, 4848, Some(40041), 10_000)
                .expect("crowbar quest cask binding");
            assert_eq!(cask.binding.transformed, 3134);
            assert_eq!(cask.first_deadline_unix_ms, Some(190_000));
        }
        for tool in ["oteryn:item.tibia.i3453", "oteryn:item.tibia.i9596"] {
            let cane = dispatch(&catalogue, tool, 5464, None, 10_000).expect("scythe cane binding");
            assert_eq!(cane.binding.yield_item, Some(5466));
        }
    }
    #[test]
    fn worldchg_voice_source_homes_choices_and_slot_identity() {
        for (draw, expected) in [
            (0, "oteryn:creature.diamond_servant"),
            (1_u64 << 62, "oteryn:creature.golden_servant"),
            (u64::MAX, "oteryn:creature.iron_servant"),
        ] {
            let sources = voice_servants(|_| draw);
            for (ordinal, (creature, home)) in sources.into_iter().enumerate() {
                assert_eq!(creature, expected);
                assert_eq!(voice_servant_ordinal(home), u8::try_from(ordinal).ok());
            }
        }
        assert_eq!(voice_servant_ordinal((33313, 31852, 8)), None);
        assert_eq!(
            voice_mage(0),
            ("oteryn:creature.mad_mage", VOICE_MAGE_HOMES[0])
        );
        assert_eq!(
            voice_mage(u64::MAX),
            ("oteryn:creature.mad_mage", VOICE_MAGE_HOMES[2])
        );
    }
}
