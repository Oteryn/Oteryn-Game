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
/// regrowth. Crystal's crowbar (register_actions.lua:884-969) only serves quest uids/actions,
/// so no crowbar terrain target is admitted.
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
    // M08B terrain pick: Crystal 201189f8 register_actions.lua:633-648 transforms the stone,
    // loose stone and loose ice piles and lets them decay back (items.xml:1002-1004 hole 594
    // 60 s, :1046-1048 hole 615 300 s, :1029-1031 hole 609 300 s). Fragile ice (:629, 7236)
    // and the 21341 pile (:637, 21342) have no decay in that items.xml, so they stay unadmitted.
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
        for undecayed in [7200, 21341] {
            assert!(dispatch(&catalogue, "oteryn:item.tibia.i3456", undecayed, None, 0).is_none());
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
