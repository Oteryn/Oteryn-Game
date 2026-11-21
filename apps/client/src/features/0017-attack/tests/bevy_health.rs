use super::*;
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, StepDirection};

#[test]
fn review_battle_icon_actor_health_draws_and_refreshes_the_same_bar()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, outbox) = super::super::tests::app()?;
    let mut retained = None;
    let mut widths = Vec::new();
    let mut colors = Vec::new();
    for hp in [75, 20] {
        outbox.bevy_hud_fixture(
            &[WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: EntityKind::Creature,
                entity: EntityRef {
                    identity: [2; 16],
                    generation: 8,
                },
                position: ActorPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                detail: EntityDetail::ActorWithIcons {
                    direction: StepDirection::South,
                    appearance_ref: 1,
                    health_percent: hp,
                    icons: Default::default(),
                },
            }],
            None,
        );
        app.update();
        let mut fills = app
            .world_mut()
            .query_filtered::<(Entity, &Node, &BackgroundColor), With<HealthFill>>();
        let (bar, node, color) = fills.single(app.world())?;
        widths.push(node.width);
        colors.push(color.0);
        assert_eq!(*retained.get_or_insert(bar), bar);
    }
    assert_eq!(widths, [percent(75.), percent(20.)]);
    assert_eq!(
        colors,
        [75, 20].map(|hp| crate::bevy_shell::health_circle::health_colour(hp, 100))
    );
    Ok(())
}
