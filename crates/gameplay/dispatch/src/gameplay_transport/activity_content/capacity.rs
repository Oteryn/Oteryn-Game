//! Fixed-slot room reserve, separate from the entry roster and native admission authority.
use crate::content::bosses::activity_template::QualifiedActivityRoomTemplate;
use crate::durability::activity_handoff::ActivityRefusal;

pub(super) fn for_template(
    template: &QualifiedActivityRoomTemplate,
) -> Result<usize, ActivityRefusal> {
    // Source callbacks and native summons need physical slots after entry.
    // The qualified template owns only its bounded captured region. Its cells
    // supply a finite reserve; no missing map or unqualified world opens it.
    let creatures = template
        .room()
        .source_world()
        .map_or(1, |world| world.cell_count().max(1));
    template
        .maximum_players()
        .checked_add(creatures)
        .filter(|capacity| {
            *capacity
                <= oteryn_gameplay_entities::carrier::CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
        })
        .ok_or(ActivityRefusal::ContentUnavailable)
}
