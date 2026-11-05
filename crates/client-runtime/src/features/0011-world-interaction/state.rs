//! Revalidate popup selections against the session stores at dispatch, not the held UI frame.
use super::{Command, Target};
use oteryn_session::{
    EntityDetail, Session, SessionStream,
    world_map::{MapOrigin, MapViewHeader},
};

pub(super) fn current<S: SessionStream>(session: &Session<S>, command: &Command) -> bool {
    let Some(selection) = &command.selection else {
        return false;
    };
    let map = session.world_map_snapshot();
    if let Some(binding) = selection.binding
        && map.as_ref().map(|map| {
            (
                map.header.content_generation,
                map.header.bundle_digest,
                map.header.reset_epoch,
            )
        }) != Some(binding)
    {
        return false;
    }
    if let Target::Creature(reference) = command.target {
        return session.world_entities().is_some_and(|entities| {
            entities.others().any(|entity| {
                entity.entity == reference
                    && entity.position == selection.position
                    && matches!(
                        entity.detail,
                        EntityDetail::Actor {
                            health_percent: 1..=100,
                            ..
                        }
                    )
            })
        });
    }
    if let Some(map) = map {
        let found = map
            .tiles
            .iter()
            .filter(|tile| tile.position == selection.position)
            .flat_map(|tile| tile.items.iter())
            .any(|item| match (&command.target, item.origin) {
                (Target::Item(handle), MapOrigin::Handle(current)) => *handle == current,
                (
                    Target::Object(object),
                    MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision,
                    },
                ) => {
                    object.expected_revision == object_revision
                        && placement(map.header, selection.position, ordinal).as_ref()
                            == Some(&object.placement)
                }
                _ => false,
            });
        if found {
            return true;
        }
    }
    session.world_entities().is_some_and(|entities| entities.others().any(|entity| {
        entity.position == selection.position && matches!((&command.target, entity.detail), (Target::Item(handle), EntityDetail::Object { item_handle: Some(current), .. }) if *handle == current)
    }))
}
fn placement(
    header: MapViewHeader,
    position: oteryn_session::ActorPosition,
    ordinal: u8,
) -> Option<Vec<u8>> {
    let x = u16::try_from(position.x).ok()?;
    let y = u16::try_from(position.y).ok()?;
    let z = u8::try_from(position.floor.checked_neg()?).ok()?;
    let key =
        (u64::from(x) << 32) | (u64::from(y) << 16) | (u64::from(z) << 8) | u64::from(ordinal);
    let mut placement = header.bundle_digest.to_vec();
    placement.extend(key.to_be_bytes());
    Some(placement)
}
