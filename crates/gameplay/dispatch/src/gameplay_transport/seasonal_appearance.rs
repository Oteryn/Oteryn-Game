//! Eleven pinned seasonal actor defaults through the existing appearance25 dictionary.
//! This source closure grants no outfit, Item, Forge stack or capability.
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_protocol_oteryn::appearance as wire;
mod source;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceLook {
    Outfit {
        look_type: u32,
        colours: [u16; 4],
        addons: u8,
    },
    Item(u16),
}
pub(super) fn npc(key: &str) -> Option<SourceLook> {
    source::NPC
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, look)| *look)
}
pub(super) fn creature(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
    let Ok(snapshot) = runtime.companion_snapshot(actor) else {
        return false;
    };
    let Some((_, expected)) = source::CREATURE
        .iter()
        .find(|(key, _)| *key == snapshot.state.policy.definition_key)
    else {
        return false;
    };
    match expected {
        SourceLook::Outfit { look_type, .. } => {
            snapshot.state.policy.outfit_appearance() == Some(*look_type)
        }
        SourceLook::Item(id) => snapshot.state.object_look_type == Some(u32::from(*id)),
    }
}
pub(super) fn npc_binding(
    runtime: &ChannelRuntimeV1,
    entity: wire::EntityRef,
    context: wire::Context,
    now: u64,
) -> Option<wire::Binding> {
    let visible = runtime.visible_entities();
    let entry = visible.npcs.iter().find(|entry| {
        entry.actor.placement_identity() == entity.identity
            && entry.actor.actor_local_generation() == entity.generation
    })?;
    let SourceLook::Outfit {
        look_type,
        colours,
        addons,
    } = npc(runtime.npc_key(entry.actor)?)?
    else {
        return None;
    };
    Some(wire::Binding {
        context,
        appearance_ref: 1,
        entity,
        root_revision: 1,
        condition_revision: entity.generation,
        owner_time_us: now,
        look: wire::Look::Outfit {
            look_type,
            colours,
            addons,
        },
        selection_revision: None,
        forge_stacks: None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_seasonal_default_looks_are_exact_three_creatures_and_eight_npcs() {
        assert_eq!(source::CREATURE.len(), 3);
        assert_eq!(source::NPC.len(), 8);
        assert_eq!(
            npc("oteryn:npc.santa_claus"),
            Some(SourceLook::Outfit {
                look_type: 160,
                colours: [0, 112, 93, 95],
                addons: 0
            })
        );
        assert_eq!(
            npc("oteryn:npc.messenger_of_santa"),
            Some(SourceLook::Outfit {
                look_type: 159,
                colours: [39, 99, 60, 117],
                addons: 0
            })
        );
        assert_eq!(
            npc("oteryn:npc.brutus_the_baker"),
            Some(SourceLook::Outfit {
                look_type: 128,
                colours: [39, 0, 0, 0],
                addons: 0
            })
        );
        assert!(npc("oteryn:npc.dawnport_guide").is_none());
        assert_eq!(
            source::CREATURE
                .iter()
                .find(|(k, _)| *k == "oteryn:creature.cake_golem")
                .map(|(_, l)| *l),
            Some(SourceLook::Item(6393))
        );
        assert_eq!(
            source::CREATURE
                .iter()
                .find(|(k, _)| *k == "oteryn:creature.the_halloween_hare")
                .map(|(_, l)| *l),
            Some(SourceLook::Outfit {
                look_type: 74,
                colours: [0; 4],
                addons: 0
            })
        );
        assert_eq!(
            source::CREATURE
                .iter()
                .find(|(k, _)| *k == "oteryn:creature.the_mutated_pumpkin")
                .map(|(_, l)| *l),
            Some(SourceLook::Outfit {
                look_type: 292,
                colours: [0; 4],
                addons: 0
            })
        );
    }
    #[test]
    fn srv7_three_seasonal_looks_match_actual_admitted_native_policies() -> Result<(), String> {
        let world =
            crate::content::native_gameplay_test_world_id().map_err(|e| format!("{e:?}"))?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or("actual active native source")?;
        for &(key, look) in source::CREATURE {
            let policy = native
                .policies()
                .policies()
                .iter()
                .find(|p| p.definition_key == key)
                .ok_or_else(|| format!("actual source policy missing {key}"))?;
            match look {
                SourceLook::Outfit { look_type, .. } => assert_eq!(
                    policy.outfit_appearance(),
                    Some(look_type),
                    "source default {key}"
                ),
                SourceLook::Item(id) => assert_eq!(
                    policy.object_look_type,
                    Some(u32::from(id)),
                    "source object default {key}"
                ),
            }
        }
        Ok(())
    }
}
