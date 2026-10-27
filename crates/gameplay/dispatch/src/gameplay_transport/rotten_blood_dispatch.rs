//! Gameplay-transport route into the existing Rotten Blood encounter owner.

use oteryn_gameplay_encounters::bosses::rotten_blood::RottenBloodEncounterOwner;
pub use oteryn_gameplay_encounters::bosses::rotten_blood::{
    AgonyDamage, RoomPosition, RoomTarget, RottenBloodEvent, RottenBloodRefusal,
};

/// Routes a creature/condition-owned event through the native encounter dispatcher.
pub fn dispatch(
    owner: &mut RottenBloodEncounterOwner,
    event: RottenBloodEvent<'_>,
) -> Result<Vec<AgonyDamage>, RottenBloodRefusal> {
    owner.dispatch(event)
}

/// Constructs the channel-local encounter owner used by the dispatch route.
pub fn owner() -> RottenBloodEncounterOwner {
    RottenBloodEncounterOwner
}
