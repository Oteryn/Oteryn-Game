//! Bounded current-world trial predicates; no counter or command grants clearance.
use super::*;

pub(super) fn area_is_clear(runtime: &ChannelRuntimeV1, callback: &Callback) -> bool {
    let Some(area) = &callback.effects.clear_area else {
        return true;
    };
    clear_complete(runtime, area)
}

fn clear_complete(runtime: &ChannelRuntimeV1, area: &world_callbacks::ClearArea) -> bool {
    candidates_clear(runtime, area, runtime.current_live_creature_census())
}

#[cfg(test)]
fn clear(runtime: &ChannelRuntimeV1, area: &world_callbacks::ClearArea, limit: usize) -> bool {
    candidates_clear(
        runtime,
        area,
        runtime.current_live_creature_candidates(limit),
    )
}

fn candidates_clear(
    runtime: &ChannelRuntimeV1,
    area: &world_callbacks::ClearArea,
    candidates: Result<Vec<crate::foundation::ExactActorRef>, crate::foundation::CarrierError>,
) -> bool {
    if area.creatures.iter().any(|key| {
        !runtime
            .companion_policy(key)
            .is_ok_and(|policy| policy.definition_key == *key)
    }) {
        return false;
    }
    let Ok(actors) = candidates else {
        return false;
    };
    actors.into_iter().all(|actor| {
        let Some(position) = runtime
            .read_actor_position(actor)
            .ok()
            .and_then(|snapshot| native(snapshot.position()))
        else {
            return false;
        };
        if !area.contains(world_callbacks::Position {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }) {
            return true;
        }
        runtime
            .creature_spell_target_atom(actor)
            .is_ok_and(|identity| !area.creatures.contains(&identity))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m07_area_clear_observes_live_carrier_and_refuses_partial_census() -> Result<(), String> {
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(39, 8);
        let policies = crate::content::native_gameplay::callback_source_native_test_policies(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_profiles.json"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_looks.json"
            )),
            runtime.content_pin().server_artifact_digest(),
        )
        .map_err(|e| format!("{e:?}"))?;
        runtime
            .install_companion_policies(policies)
            .map_err(|e| format!("{e:?}"))?;
        let area = world_callbacks::ClearArea {
            minimum: Position {
                x: 100,
                y: 100,
                floor: -3,
            },
            maximum: Position {
                x: 110,
                y: 110,
                floor: -3,
            },
            additional_floors: Vec::new(),
            creatures: vec!["oteryn:creature.damned_soul".into()],
        };
        assert!(clear(&runtime, &area, 8));
        let mut unknown = area.clone();
        unknown.creatures = vec!["oteryn:creature.m07_unknown_fixture".into()];
        assert!(
            !clear(&runtime, &unknown, 8),
            "unknown creature definitions cannot prove an empty room"
        );
        let outside = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 111,
                    y: 100,
                    floor: 3,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            clear(&runtime, &area, 8),
            "creatures outside the exact room do not block it"
        );
        assert!(
            !clear(&runtime, &area, 0),
            "a partial census never proves clearance"
        );
        let boundary = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 110,
                    y: 110,
                    floor: 3,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            !clear(&runtime, &area, 8),
            "inclusive room boundary is still occupied"
        );
        assert!(runtime.kill_creature_for_test(boundary));
        assert!(
            clear(&runtime, &area, 8),
            "dead carrier entries do not count as live foes"
        );
        let upper = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 105,
                    y: 105,
                    floor: 4,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            clear(&runtime, &area, 8),
            "legacy census stays single-floor"
        );
        let mut stacked = area.clone();
        stacked.additional_floors = vec![-4];
        assert!(
            !clear(&runtime, &stacked, 8),
            "a live guard on an opted-in floor blocks clearance"
        );
        assert!(runtime.kill_creature_for_test(upper));
        assert!(clear(&runtime, &stacked, 8));
        runtime
            .remove_encounter_actor(outside)
            .map_err(|e| format!("{e:?}"))?;
        assert!(clear(&runtime, &area, 8));
        Ok(())
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn m07_area_clear_observes_live_carrier_and_refuses_partial_census() -> Result<(), String> {
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(39, 8);
        let policies = crate::content::native_gameplay::callback_source_native_test_policies(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_profiles.json"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_looks.json"
            )),
            runtime.content_pin().server_artifact_digest(),
        )
        .map_err(|e| format!("{e:?}"))?;
        runtime
            .install_companion_policies(policies)
            .map_err(|e| format!("{e:?}"))?;
        let area = world_callbacks::ClearArea {
            additional_floors: Vec::new(),
            minimum: Position {
                x: 100,
                y: 100,
                floor: -3,
            },
            maximum: Position {
                x: 110,
                y: 110,
                floor: -3,
            },
            creatures: vec!["oteryn:creature.damned_soul".into()],
        };
        assert!(clear(&runtime, &area, 8));
        let mut unknown = area.clone();
        unknown.creatures = vec!["oteryn:creature.m07_unknown_fixture".into()];
        assert!(
            !clear(&runtime, &unknown, 8),
            "unknown creature definitions cannot prove an empty room"
        );
        let outside = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 111,
                    y: 100,
                    floor: 3,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            clear(&runtime, &area, 8),
            "creatures outside the exact room do not block it"
        );
        assert!(
            !clear(&runtime, &area, 0),
            "a partial census never proves clearance"
        );
        let boundary = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 110,
                    y: 110,
                    floor: 3,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            !clear(&runtime, &area, 8),
            "inclusive room boundary is still occupied"
        );
        assert!(runtime.kill_creature_for_test(boundary));
        assert!(
            clear(&runtime, &area, 8),
            "dead carrier entries do not count as live foes"
        );
        runtime
            .remove_encounter_actor(outside)
            .map_err(|e| format!("{e:?}"))?;
        assert!(clear(&runtime, &area, 8));
        Ok(())
    }
}

#[cfg(test)]
mod population_tests {
    use super::*;

    #[test]
    fn trial_clearance_ignores_more_than_4096_live_creatures_outside_the_room() -> Result<(), String>
    {
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(39, 4_104);
        let policies = crate::content::native_gameplay::callback_source_native_test_policies(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_profiles.json"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/src/callback_native_child_looks.json"
            )),
            runtime.content_pin().server_artifact_digest(),
        )
        .map_err(|e| format!("{e:?}"))?;
        runtime
            .install_companion_policies(policies)
            .map_err(|e| format!("{e:?}"))?;
        let area = world_callbacks::ClearArea {
            minimum: Position {
                x: 100,
                y: 100,
                floor: -3,
            },
            maximum: Position {
                x: 110,
                y: 110,
                floor: -3,
            },
            additional_floors: Vec::new(),
            creatures: vec!["oteryn:creature.damned_soul".into()],
        };
        for _ in 0..4_097 {
            runtime
                .admit_encounter_actor(
                    "oteryn:creature.damned_soul",
                    100,
                    MovementLocalPosition {
                        x: 111,
                        y: 100,
                        floor: 3,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
        }
        assert!(
            clear_complete(&runtime, &area),
            "unrelated live creatures must not exhaust trial clearance"
        );
        let inside = runtime
            .admit_encounter_actor(
                "oteryn:creature.damned_soul",
                100,
                MovementLocalPosition {
                    x: 105,
                    y: 105,
                    floor: 3,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            !clear_complete(&runtime, &area),
            "one live trial creature must still refuse clearance"
        );
        assert!(runtime.kill_creature_for_test(inside));
        assert!(clear_complete(&runtime, &area));
        Ok(())
    }
}
