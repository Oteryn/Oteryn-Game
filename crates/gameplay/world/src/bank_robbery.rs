//! Normal-map Bank Robbery placement through the existing Channel raid carrier.
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::{CarrierError, ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_durability::durability::world_event::bank_robbery::{self, City, Projection};
use oteryn_gameplay_entities::carrier::{
    EncounterSpawnError, NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome,
};

fn bodies(city: &City) -> impl Iterator<Item = (u16, &str, [u16; 3])> {
    // Admit the escort before making the robber attackable. A transient
    // obstruction can defer any placement until the next supervision tick.
    city.escort
        .iter()
        .flat_map(|escort| {
            (1_u16..)
                .zip(escort.cells.iter().copied())
                .map(|(entry, cell)| (entry, escort.creature.as_str(), cell))
        })
        .chain(std::iter::once((0, city.robber.as_str(), city.cell)))
}

fn walkable(cells: &NativeEntryMovementCells, [x, y, z]: [u16; 3]) -> bool {
    let cell = LogicalCell {
        x: i32::from(x),
        y: i32::from(y),
        z: i32::from(z),
    };
    cells.index().lookup(cells.scope(), cell).ok() == Some(CollisionClass::Walkable)
}

/// Loaded, enterable cells are required for a Channel to claim projection.
pub fn loaded(
    cells: &NativeEntryMovementCells,
) -> Result<[bool; 4], crate::durability::DurabilityError> {
    Ok(bank_robbery::cities()?
        .each_ref()
        .map(|city| bodies(city).all(|(_, _, cell)| walkable(cells, cell))))
}

/// Realize only a committed occurrence for this Channel's current runtime.
pub fn project(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    robbery: Projection,
) -> Result<(), EncounterSpawnError> {
    let binding = runtime.binding();
    if !robbery.matches(binding.runtime_scope(), binding.scope_generation()) {
        return Err(EncounterSpawnError::Native(CarrierError::WrongScope));
    }
    let stamp = runtime
        .issue_owner_work()
        .map_err(EncounterSpawnError::Native)?;
    for (entry, creature, cell) in bodies(robbery.city) {
        let to_error = EncounterSpawnError::Native;
        let policy = runtime.companion_policy(creature).map_err(to_error)?;
        let spec = NativeRaidSpawnSpec::qualified(
            creature,
            &policy.definition_revision,
            runtime.content_pin().server_artifact_digest(),
            entry == 0,
        )
        .map_err(to_error)?;
        let [x, y, z] = cell;
        let at = MovementLocalPosition {
            x: i32::from(x),
            y: i32::from(y),
            floor: i16::try_from(z).map_err(|_| to_error(CarrierError::InvalidSpawnDefinition))?,
        };
        let placed = runtime.realize_retryable_raid_spawn(
            stamp,
            RaidSpawnIdentity {
                firing_id: robbery.firing_id,
                wave: bank_robbery::RAID_WAVE,
                entry,
                index: 0,
            },
            &spec,
            at,
            |_, _| Some(walkable(cells, cell)),
        )?;
        if placed.outcome == RaidSpawnOutcome::Failed {
            break;
        }
    }
    Ok(())
}
