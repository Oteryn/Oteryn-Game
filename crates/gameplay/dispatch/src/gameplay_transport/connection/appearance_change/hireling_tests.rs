#![allow(clippy::expect_used, reason = "bounded appearance dictionary fixture")]
use super::*;
use crate::gameplay_transport::appearance_change::observation::Observation;
use crate::gameplay_transport::world_spatial::{ActorPosition, StepDirection};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityKind, WorldSpatialEntity,
};

fn spatial(id: u8) -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind: EntityKind::Npc,
        entity: WireEntityRef {
            identity: [id; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: i32::from(id),
            y: 0,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 0,
            health_percent: 100,
        },
    }
}
fn observation(
    entity: WireEntityRef,
    revision: u64,
    look_type: u32,
    colors: [u8; 4],
) -> Observation {
    let context = wire::Context {
        content_generation: [1; 32],
        cosmetics_digest: [2; 32],
    };
    Observation {
        catalogue: wire::Catalogue {
            context,
            root_revision: 1,
            selected: None,
            palette: vec![[0, 0, 0]; 133],
            outfits: Vec::new(),
            mounts: Vec::new(),
        },
        bindings: vec![wire::Binding {
            context,
            appearance_ref: 1,
            entity,
            root_revision: 1,
            condition_revision: revision,
            owner_time_us: 1,
            look: wire::Look::Outfit {
                look_type,
                colours: colors.map(u16::from),
                addons: 0,
            },
            selection_revision: None,
            forge_stacks: None,
        }],
    }
}

#[test]
fn hireling_dictionary_attaches_existing_npc_look_once_and_does_not_bind_static_npcs() {
    let channel = ChannelEntities {
        content_generation: [1; 32],
        observer: [9; 16],
        entities: vec![ChannelEntity {
            kind: VisibleKind::Hireling,
            health_percent: 100,
            direction: StepDirection::South,
            identity: [1; 16],
            generation: 1,
            position: ActorPosition {
                x: 1,
                y: 0,
                floor: 7,
            },
            revision: 1,
        }],
        lights: EntityLights::new(),
        observed_at_ms: 1,
    };
    let mut dictionary = SessionAppearance::default();
    dictionary.select_hirelings(&channel);
    let mut actors = vec![spatial(1), spatial(2)];
    let refs = dictionary.players(&actors);
    assert_eq!(refs, vec![actors[0].entity]);
    let initial = dictionary
        .upserts(observation(refs[0], 1, 1110, [1, 2, 3, 4]), &refs)
        .expect("initial mappings");
    assert_eq!(initial.len(), 1);
    let reference = initial[0].appearance_ref;
    assert_ne!(reference, 0);
    assert!(
        dictionary
            .upserts(observation(refs[0], 1, 1110, [1, 2, 3, 4]), &refs)
            .expect("replay")
            .is_empty()
    );
    let changed = dictionary
        .upserts(observation(refs[0], 2, 1126, [5, 6, 7, 8]), &refs)
        .expect("changed outfit");
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].appearance_ref, reference);
    assert_eq!(
        changed[0].look,
        wire::Look::Outfit {
            look_type: 1126,
            colours: [5, 6, 7, 8],
            addons: 0
        }
    );
    dictionary.attach(&mut actors).expect("attach");
    assert!(
        matches!(actors[0].detail,EntityDetail::Actor{appearance_ref,..} if appearance_ref==reference)
    );
    assert!(matches!(
        actors[1].detail,
        EntityDetail::Actor {
            appearance_ref: 0,
            ..
        }
    ));
}
