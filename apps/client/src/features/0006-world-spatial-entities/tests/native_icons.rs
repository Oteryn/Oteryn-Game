use super::*;

#[test]
fn srx3_native_creature_icons_decode_into_native_labels_and_clear_on_removal()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::test_support::wire::world_spatial_entities as wire;
    let at = oteryn_session::ActorPosition {
        x: 100,
        y: 200,
        floor: 0,
    };
    let own = oteryn_session::EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let mut scene = Scene::centered_on(
        Arc::new(World::builtin()?),
        None,
        TileCoord::new(100, 200),
        StepDir::South,
        &[],
    )?;
    let mut entity = wire::WorldSpatialEntity {
        display_name: None,
        kind: wire::EntityKind::Player,
        entity: own,
        position: at,
        detail: wire::EntityDetail::ActorWithIcons {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
            icons: wire::NativeCreatureIcons::new(&[
                wire::NativeCreatureIcon {
                    category: 0,
                    id: 2,
                    count: 0,
                },
                wire::NativeCreatureIcon {
                    category: 0,
                    id: 2,
                    count: 111,
                },
                wire::NativeCreatureIcon {
                    category: 0,
                    id: 17,
                    count: 100,
                },
            ])
            .map_err(|error| format!("{error:?}"))?,
        },
    };
    let fields = wire::EntityFields::selected(&[6, wire::CAPABILITY_NATIVE_CREATURE_ICONS_V1]);
    for visible in [true, false] {
        if !visible {
            entity.detail = wire::EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            };
        }
        let snapshot = wire::WorldSpatialEntitiesSnapshot {
            content_generation: [0; 32],
            actor_position: at,
            own_identity: own.identity,
            entities: vec![entity],
        };
        let bytes = wire::encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            fields,
            &wire::EntityLights::new(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let (decoded, _) = wire::decode_world_spatial_entities_snapshot_with_fields(&bytes, fields)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decoded.entities[0].detail, entity.detail,
            "native labels must consume decoded actual field14, including visible zero and duplicate icon IDs"
        );
        scene.set_presentations(
            &[VisibleAppearance {
                forge_stacks: None,
                light: None,
                entity: decoded.entities[0],
                look: None,
            }],
            0,
        )?;
        assert_eq!(
            scene.name_count(),
            usize::from(visible),
            "native icon-only label must appear without a creature name and disappear when full source list clears"
        );
        if visible {
            let expected = scene.cache.name("[Cross 0] [Cross 111] [Up 100]");
            assert_eq!(
                scene.names[0].1, expected,
                "native glyph label must retain both RedCross source entries, visible zero, exact counts and source ordering"
            );
            assert!(
                scene.sprites().len() >= expected.len(),
                "labels must create native glyph quads"
            );
        }
    }
    Ok(())
}

#[path = "native_icons_play.rs"]
mod play;
