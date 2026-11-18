//! Combat observations against the bundle spawn approached by the World journey.
//! Missing qualification setup is a failure, never a missing-content skip.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};

/// Keep all verdicts, including prerequisite failures, until the phase ends.
#[derive(Default)]
pub(super) struct Checks {
    rows: std::collections::BTreeMap<&'static str, [usize; 3]>,
}

impl Checks {
    pub(super) fn skip(&mut self, row: &'static str, case: &str, missing: &str) {
        self.rows.entry(row).or_default()[2] += 1;
        println!("COMBAT_CHECK {row} {case} SKIPPED content:{missing}");
    }

    pub(super) fn record(&mut self, row: &'static str, case: &str, result: TestResult) {
        match result {
            Ok(()) => {
                self.rows.entry(row).or_default()[0] += 1;
                println!("COMBAT_CHECK {row} {case} PASS observed_in_protocol");
            }
            Err(error) => self.record_failure(row, case, error.as_ref()),
        }
    }

    pub(super) fn record_failure(
        &mut self,
        row: &'static str,
        case: &str,
        error: &(dyn std::error::Error + 'static),
    ) {
        let cause = super::client_world::verdict::classify(error).suffix(&format!("{row}/{case}"));
        let reason = error.to_string().replace(['\n', '\r'], " ");
        self.rows.entry(row).or_default()[1] += 1;
        println!("COMBAT_CHECK {row} {case} FAIL {reason}{cause}");
    }

    pub(super) fn finish(self) -> TestResult {
        let mut failures = 0;
        for (row, [pass, fail, skipped]) in self.rows {
            println!("COMBAT_SUMMARY row={row} pass={pass} fail={fail} skipped={skipped}");
            failures += fail + skipped;
        }
        if failures == 0 {
            Ok(())
        } else {
            Err(format!("combat journey: {failures} failed checks").into())
        }
    }
}

pub(super) async fn combat_journey(
    client: &mut DevClientSession,
    world: &crate::map::boot::BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    url: &str,
) -> TestResult {
    let mut checks = Checks::default();
    for (name, fight_mode) in [
        ("offensive", FightMode::Offensive),
        ("balanced", FightMode::Balanced),
        ("defensive", FightMode::Defensive),
    ] {
        for (chase_name, chase) in [("stand", ChaseMode::Stand), ("chase", ChaseMode::Chase)] {
            let result = async {
                let wanted = client_world::travel_to_spawn_target(
                    client,
                    world,
                    spawns,
                    "oteryn:creature.rat",
                )
                .await?;
                melee_damage(
                    client,
                    "Rat",
                    FightModes {
                        fight_mode,
                        chase,
                        secure: true,
                    },
                    wanted,
                    url,
                )
                .await
            }
            .await;
            checks.record("C01", &format!("{name}-{chase_name}"), result);
        }
    }
    checks.finish()
}

async fn melee_damage(
    client: &mut DevClientSession,
    name: &str,
    modes: FightModes,
    wanted: oteryn_dev_client::EntityRef,
    url: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;
    if client.inventory().is_none_or(|inventory| {
        !inventory.equipment.iter().any(|entry| {
            matches!(
                entry.slot,
                EquipmentSlot::RightHand | EquipmentSlot::LeftHand
            )
        })
    }) {
        return Err("qualification bootstrap has no equipped melee weapon".into());
    }
    let outcome = client.session_mut().set_fight_modes(&modes).await?;
    if outcome.disposition != AttackIntentDisposition::Ok {
        return Err(format!("fight modes refused: {outcome:?}").into());
    }
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Creature
                    && client_world::has_display_name(entity, name)
                    && entity.entity == wanted
                    && entity.position.floor == here.floor
                    && entity
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entity.position.y.abs_diff(here.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("bundle spawn has no adjacent live monster")?;
    client_hunt::kill(client, target, url, Duration::from_secs(60)).await?;
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes != modes)
    {
        return Err("server combat state does not retain the requested modes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_journey_accumulates_failures_and_does_not_qualify_skips() {
        let mut checks = Checks::default();
        checks.record("C01", "offensive-stand", Err("first failed check".into()));
        checks.record("C01", "balanced-stand", Ok(()));
        checks.record("C02", "bow-ammo", Err("second failed check".into()));
        assert_eq!(checks.rows["C01"], [1, 1, 0]);
        assert!(checks.finish().is_err());
        let mut skipped = Checks::default();
        skipped.skip("C02", "bow-ammo", "oteryn:item.tibia.i3350");
        assert!(skipped.finish().is_err());
    }
}
