use oteryn_gameplay_entities::*;
pub trait TentacleCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn commit_tentacle_next(
        &mut self,
        ledger: &mut TentacleDeathLedger,
        actor: ExactActorRef,
    ) -> Result<TentacleNextResult, CarrierError>;
}
// Append beside native physical carrier. Ten exact source callbacks, not an Encounter interpreter.
#[derive(Debug, Default)]
pub struct TentacleDeathLedger {
    entries: Vec<(CreatureDeathOccurrenceKey, Option<ExactActorRef>)>,
}
#[derive(Debug, Clone, Copy)]
pub struct TentacleNextResult {
    pub newly_committed: bool,
    pub spawned: Option<ExactActorRef>,
    pub position: Option<MovementLocalPosition>,
}
fn crystal_tentacle_line(key: &[u8]) -> Result<&'static [MovementLocalPosition], CarrierError> {
    Ok(match key {
        b"oteryn:creature.tentacle" => &[
            MovementLocalPosition {
                x: 33723,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33724,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33725,
                y: 31180,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle2" => &[
            MovementLocalPosition {
                x: 33723,
                y: 31186,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31187,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31188,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31189,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle3" => &[
            MovementLocalPosition {
                x: 33727,
                y: 31184,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33727,
                y: 31185,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle4" => &[
            MovementLocalPosition {
                x: 33731,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33732,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33733,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33734,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33735,
                y: 31180,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle5" => &[
            MovementLocalPosition {
                x: 33718,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31179,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31178,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle6" => &[
            MovementLocalPosition {
                x: 33714,
                y: 31180,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33714,
                y: 31179,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33714,
                y: 31178,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle7" => &[
            MovementLocalPosition {
                x: 33726,
                y: 31182,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31181,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31180,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31179,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31178,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle8" => &[
            MovementLocalPosition {
                x: 33718,
                y: 31187,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31188,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31189,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31190,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle9" => &[
            MovementLocalPosition {
                x: 33714,
                y: 31185,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33713,
                y: 31185,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33712,
                y: 31185,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle10" => &[
            MovementLocalPosition {
                x: 33716,
                y: 31181,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33716,
                y: 31182,
                floor: 6,
            },
        ],
        _ => return Err(CarrierError::CreatureTargetMismatch),
    })
}
impl TentacleCarrierExt for ChannelRuntimeV1 {
    /// Callable only after the root source-specific Map owner removes the line prop.
    /// Data-loader caller must qualify current exact source profile health2500 and map frame.
    /// Creates the existing same species through native admission+initialize; no synthetic identity.
    /// Final-tile transform/removal and ENERGYHIT remain independent WorldObject/presentation work.
    fn commit_tentacle_next(
        &mut self,
        ledger: &mut TentacleDeathLedger,
        actor: ExactActorRef,
    ) -> Result<TentacleNextResult, CarrierError> {
        let owner = self.borrow_combat_death();
        let key = owner.creature_target_identity(actor)?;
        let line = crystal_tentacle_line(key)?;
        let identity = std::str::from_utf8(key)
            .map_err(|_| CarrierError::CreatureTargetMismatch)?
            .to_owned();
        let (death, position) = owner.projected_death(actor)?;
        let next = line
            .iter()
            .position(|p| *p == position)
            .map_or(0, |index| index + 1);
        let at = line.get(next).copied();
        ledger
            .entries
            .retain(|(key, _)| self.encounter_death_is_current(*key));
        if let Some((_, spawned)) = ledger.entries.iter().find(|(key, _)| *key == death) {
            return Ok(TentacleNextResult {
                newly_committed: false,
                spawned: *spawned,
                position: at,
            });
        }
        ledger
            .entries
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let spawned = if let Some(at) = at {
            Some(self.admit_encounter_actor(&identity, 2500, at)?)
        } else {
            None
        };
        ledger.entries.push((death, spawned));
        Ok(TentacleNextResult {
            newly_committed: true,
            spawned,
            position: at,
        })
    }
}

#[cfg(test)]
#[path = "native_death_tests.rs"]
mod tests;
