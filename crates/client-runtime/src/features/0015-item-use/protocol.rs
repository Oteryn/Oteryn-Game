//! Encode use-with through the existing item/rune command, preserving server identities.
use crate::world_interaction::{Target, world_object};
pub use oteryn_session::ActorPosition;
use oteryn_session::{ItemHandle, SessionError, world_context::SpellTargetPosition};

pub fn use_with_payload(source: ItemHandle, target: &Target) -> Result<Vec<u8>, SessionError> {
    let mut intent = world_object::UseIntent {
        target: world_object::UseTarget::Item(source),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    match target {
        Target::Creature(entity) => intent.use_with = Some(*entity),
        Target::Item(item) => {
            intent.use_with_map_item = Some(world_object::MapUseTarget::Item(*item))
        }
        Target::Object(object) => {
            intent.use_with_map_item = Some(world_object::MapUseTarget::Object(object.clone()));
        }
        Target::Position(position) => {
            intent.use_with_position = Some(SpellTargetPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            });
        }
    }
    world_object::encode_item_use_intent(&intent).map_err(|_| SessionError::SessionUnusable)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn use_with_preserves_source_and_each_destination() -> Result<(), SessionError> {
        let source = ItemHandle::MAX;
        for target in [
            Target::Item(ItemHandle::MIN),
            Target::Creature(oteryn_session::EntityRef {
                identity: [7; 16],
                generation: 9,
            }),
            Target::Object(world_object::WorldObjectTarget {
                placement: vec![3; 40],
                expected_revision: 12,
            }),
            Target::Position(oteryn_session::ActorPosition {
                x: -1,
                y: 200,
                floor: -7,
            }),
        ] {
            let payload = use_with_payload(source, &target)?;
            let intent = world_object::decode_use_intent_target(&[4, 6, 11, 15, 18, 22], &payload)
                .map_err(|_| SessionError::SessionUnusable)?;
            assert_eq!(intent.target, world_object::UseTarget::Item(source));
            match target {
                Target::Item(item) => assert_eq!(
                    intent.use_with_map_item,
                    Some(world_object::MapUseTarget::Item(item))
                ),
                Target::Object(object) => assert_eq!(
                    intent.use_with_map_item,
                    Some(world_object::MapUseTarget::Object(object))
                ),
                Target::Creature(entity) => assert_eq!(intent.use_with, Some(entity)),
                Target::Position(position) => assert_eq!(
                    intent.use_with_position,
                    Some(SpellTargetPosition {
                        x: position.x,
                        y: position.y,
                        floor: position.floor
                    })
                ),
            }
        }
        Ok(())
    }
}
