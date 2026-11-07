//! Static object effects share the existing Channel map owner, never the item writer.
use super::*;
use crate::map::facts::QuestObjectMutation;
use crate::map::view::{EntryFacts, EntrySource};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn apply_callback_objects(
        &self,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: EntryFacts,
    ) -> UseDisposition {
        let Some(objects) = &callback.objects else {
            return UseDisposition::Rejected;
        };
        let mut operations = Vec::new();
        if objects.consume_trigger {
            let Some((position, ordinal)) = crate::map::view::placement_of(key) else {
                return UseDisposition::Rejected;
            };
            if after.pickupable {
                return UseDisposition::Rejected;
            }
            operations.push(QuestObjectMutation {
                position,
                remove: Some(EntrySource::Base {
                    ordinal,
                    placement_key: key,
                    movable: false,
                }),
                create: None,
            });
        }
        for mutation in &objects.mutations {
            let position = tile(mutation.position);
            if map.facts.house_tile(position) {
                return UseDisposition::Rejected;
            }
            let Ok(Some(current)) =
                crate::map::view::compose_tile(&map.overlay, &map.facts, position)
            else {
                return UseDisposition::Rejected;
            };
            let mut candidates = current
                .entries
                .iter()
                .filter(|entry| Some(entry.facts.appearance_id) == mutation.remove);
            let removed = candidates.next();
            if candidates.next().is_some()
                || removed.is_some_and(|entry| {
                    entry.facts.pickupable
                        || matches!(entry.source, EntrySource::Ground(_) | EntrySource::House(_))
                })
            {
                return UseDisposition::Rejected;
            }
            let create = if let Some(appearance) = mutation.create {
                let Some(facts) = map.facts.quest_object_facts(appearance) else {
                    return UseDisposition::Rejected;
                };
                // Floor-changing objects need the composed floor resolver; do not
                // publish a visual-only hole before that source path is qualified.
                if crate::map::floor::palette_mask(
                    oteryn_world_bundle::bundle::Family::Terrain,
                    &format!("oteryn:terrain.tibia.i{appearance}"),
                ) != 0
                {
                    return UseDisposition::Rejected;
                }
                (!current
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == appearance))
                .then_some(facts)
            } else {
                None
            };
            operations.push(QuestObjectMutation {
                position,
                remove: removed.map(|entry| entry.source),
                create,
            });
        }
        if map
            .facts
            .apply_quest_objects(key, revision, after, &operations)
        {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        }
    }
}
