//! Durable, content-qualified terrain-tool transformations.
//!
//! The network owner resolves the carried tool and map placement before calling this module.
//! This module deliberately has no numeric fallback: only catalogue entries admitted by the
//! active content generation can produce a write plan.

use std::time::Duration;

/// A tool identity from the pinned item catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTool {
    Shovel,
    Pick,
    Machete,
    Scythe,
    Sickle,
    Crowbar,
}

impl TerrainTool {
    /// Resolve the Crystal-compatible tool from its qualified Oteryn production key.
    pub fn from_production_key(key: &str) -> Option<Self> {
        Some(match key {
            "oteryn:item.tibia.i3457" | "oteryn:item.tibia.i5710" => Self::Shovel,
            "oteryn:item.tibia.i3456" => Self::Pick,
            // Global permits both machetes and the three Secret Agent Tools.  Crystal's
            // shared onUseMachete path is also reached by tool gear (9594/9596/9598).
            "oteryn:item.tibia.i3308"
            | "oteryn:item.tibia.i3330"
            | "oteryn:item.tibia.i9594"
            | "oteryn:item.tibia.i9596"
            | "oteryn:item.tibia.i9598" => Self::Machete,
            "oteryn:item.tibia.i3453" => Self::Scythe,
            "oteryn:item.tibia.i3293" => Self::Sickle,
            "oteryn:item.tibia.i3304" => Self::Crowbar,
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

/// The squeezing gear of girlpower. Crystal data/scripts/actions/tools/squeezing_gear_of_girlpower.lua
/// :19-31 tries the scythe, sickle, rope, shovel, pick and machete in that order; the targets of
/// those tools are disjoint, so the first matching binding is the donor's. The 1% jam roll at
/// :16-17 and :34-40 needs an item transform and decay owner and is not applied here.
const TOOL_GEAR: &str = "oteryn:item.tibia.i9596";
const TOOL_GEAR_TOOLS: [TerrainTool; 5] = [
    TerrainTool::Scythe,
    TerrainTool::Sickle,
    TerrainTool::Shovel,
    TerrainTool::Pick,
    TerrainTool::Machete,
];

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
        (binding.tool == tool || (tool_key == TOOL_GEAR && TOOL_GEAR_TOOLS.contains(&binding.tool)))
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

/// A harvest the gathering craft owner commits: the terrain CAS, its decay chain and the yield
/// in one inventory transaction (`GatheringCraft`). Crystal creates the yield on the target
/// tile; Oteryn mints it into the carried backpack through the existing audited craft mint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestPlan {
    pub attempt: oteryn_gameplay_items::fishing::GatheringAttempt,
    /// Successive states and delays from the preceding durable deadline, milliseconds.
    pub decay: Vec<(u32, i64)>,
}

/// The harvest binding of one carried tool and target state, if the catalogue has a yield for it.
pub fn harvest_plan(tool_key: &str, target_state: u32) -> Option<HarvestPlan> {
    let catalogue = crystal_core_bindings();
    let plan = dispatch(&catalogue, tool_key, target_state, None, 0)?;
    let binding = plan.binding;
    let yield_item = u16::try_from(binding.yield_item?).ok()?;
    Some(HarvestPlan {
        attempt: oteryn_gameplay_items::fishing::GatheringAttempt {
            after: Some(u16::try_from(binding.transformed).ok()?),
            yield_item: Some(yield_item),
            consumes_bait: false,
        },
        decay: binding
            .decay
            .iter()
            .map(|step| Some((step.state, i64::try_from(step.after.as_millis()).ok()?)))
            .collect::<Option<_>>()?,
    })
}

/// Bounded mappings. Digging and jungle grass are backed by Canary register_actions.lua at
/// 47dfd51f and the pinned items.xml. Harvest plans are committed by the gathering craft owner,
/// which mints their yield (`harvest_plan`). The sickle plan is Crystal 201189f8
/// sickle.lua:12-15 (5463 to 5462, 5466 created on the target tile) with items.xml:14996-15027
/// regrowth. Crystal data-crystal/scripts/lib/register_actions.lua:448-452 delegates
/// crowbar use to destroyItem, with no terrain-specific target, so none is admitted here.
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
        // Crystal data-global/scripts/lib/register_actions.lua:1040-1044: the scythe on burning
        // sugar cane 5464 -> 5463 and creates 5466; items.xml:15002-15005 gives 5463 no decay.
        TerrainToolBinding {
            tool: TerrainTool::Scythe,
            target: 5464,
            required_action: None,
            transformed: 5463,
            yield_item: Some(5466),
            decay: Vec::new(),
        },
        // data-global register_actions.lua:1049-1052; items.xml:57736 gives reed no decay.
        TerrainToolBinding {
            tool: TerrainTool::Scythe,
            target: 30623,
            required_action: None,
            transformed: 30624,
            yield_item: Some(30975),
            decay: Vec::new(),
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
    bindings.extend([(7200, 7236), (21341, 21342)].map(|(target, transformed)| {
        TerrainToolBinding {
            tool: TerrainTool::Pick,
            target,
            required_action: None,
            transformed,
            yield_item: None,
            decay: Vec::new(),
        }
    }));
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
    bindings
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
            ("oteryn:item.tibia.i3293", 5463, 5462),
            ("oteryn:item.tibia.i3456", 593, 594),
            ("oteryn:item.tibia.i3456", 606, 615),
            ("oteryn:item.tibia.i3456", 608, 609),
            ("oteryn:item.tibia.i3293", 9020, 9021),
            ("oteryn:item.tibia.i3304", 9010, 9011),
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

    /// M08 (CLOSE-2): Crystal data-global/scripts/lib/register_actions.lua:1040-1049 (scythe on
    /// burning sugar cane 5464 -> 5463 + 5466, on wheat 3653 -> 3651 + 3605, both
    /// `Game.createItem(.., 1, toPosition)`), data/scripts/actions/tools/sickle.lua:4-8 and
    /// items.xml:11551-11565 (wheat 240 s + 240 s), :15002-15027 (5463 no decay; 5462 240 s).
    #[test]
    fn m08_harvest_plans_carry_the_donor_yield_and_decay_chain() {
        let wheat = harvest_plan("oteryn:item.tibia.i3453", 3653).expect("wheat harvest");
        assert_eq!(wheat.attempt.after, Some(3651));
        assert_eq!(wheat.attempt.yield_item, Some(3605));
        assert!(!wheat.attempt.consumes_bait);
        assert_eq!(wheat.decay, [(3652, 240_000), (3653, 240_000)]);
        let burning = harvest_plan("oteryn:item.tibia.i3453", 5464).expect("burning cane");
        assert_eq!(burning.attempt.after, Some(5463));
        assert_eq!(burning.attempt.yield_item, Some(5466));
        assert!(burning.decay.is_empty());
        let sickle = harvest_plan("oteryn:item.tibia.i3293", 5463).expect("cane");
        assert_eq!(sickle.attempt.after, Some(5462));
        assert_eq!(sickle.attempt.yield_item, Some(5466));
        assert_eq!(sickle.decay, [(5470, 240_000), (5465, 240_000)]);
        // A yield-free terrain binding, the wrong tool and an unknown target are not harvests.
        assert!(harvest_plan("oteryn:item.tibia.i3456", 593).is_none());
        assert!(harvest_plan("oteryn:item.tibia.i3293", 3653).is_none());
        assert!(harvest_plan("oteryn:item.tibia.i3453", 3651).is_none());
    }

    /// M08 follow-up: Crystal data/scripts/actions/tools/squeezing_gear_of_girlpower.lua:19-31
    /// tries onUseScythe, onUseSickle, onUseRope, onUseShovel, onUsePick, onUseMachete for 9596;
    /// data-global/scripts/lib/register_actions.lua:1049-1052 gives the scythe reed 30623 ->
    /// 30624 + 30975 (items.xml:57736 has no duration for either reed state). The golden sickle
    /// 3306 appears only in the unregistered data-crystal list: sickle.lua:14 registers 3293 only.
    #[test]
    fn m08_tool_gear_resolves_every_donor_tool_and_reed_is_harvested() {
        let gear = "oteryn:item.tibia.i9596";
        let catalogue = crystal_core_bindings();
        for (target, transformed) in [
            (3653, 3651),
            (5464, 5463),
            (5463, 5462),
            (867, 868),
            (593, 594),
            (3696, 3695),
        ] {
            let plan = dispatch(&catalogue, gear, target, None, 0).expect("gear binding");
            assert_eq!(plan.binding.transformed, transformed, "{target}");
        }
        assert_eq!(
            harvest_plan(gear, 3653),
            harvest_plan("oteryn:item.tibia.i3453", 3653)
        );
        for tool in ["oteryn:item.tibia.i3453", gear] {
            let reed = harvest_plan(tool, 30623).expect("reed harvest");
            assert_eq!(reed.attempt.after, Some(30624));
            assert_eq!(reed.attempt.yield_item, Some(30975));
            assert!(reed.decay.is_empty());
        }
        assert!(harvest_plan("oteryn:item.tibia.i3293", 30623).is_none());
        assert!(harvest_plan("oteryn:item.tibia.i3306", 5464).is_none());
        // The plain machete keeps only the machete targets.
        assert!(dispatch(&catalogue, "oteryn:item.tibia.i3308", 3653, None, 0).is_none());
    }

    #[test]
    fn m08b_pick_piles_decay_back_on_the_donor_clock_and_crowbar_has_no_terrain_target() {
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
        assert!(
            catalogue
                .iter()
                .all(|binding| binding.tool != TerrainTool::Crowbar)
        );
    }
}
