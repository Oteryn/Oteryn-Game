#[test]
fn o021_live_health_and_party_marks_project_at_entity_position()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let mut json = serde_json::to_value(play.preferences())?;
    json["creature_bars"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    let entity = oteryn_session::EntityRef {
        identity: [8; 16],
        generation: 2,
    };
    play.entities.push(oteryn_session::WorldSpatialEntity {
        entity,
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        position: oteryn_session::ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 50,
        },
    });
    play.rebuild()?;
    let baseline = play.scene().sprites().len();
    json["creature_bars"] = serde_json::json!(true);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline + 2);
    play.party_icons = vec![oteryn_client_runtime::party_skull::PartyIcon {
        entity,
        shield: 3,
        skull: 4,
    }];
    play.rebuild()?;
    assert!(play.scene().sprites().len() > baseline + 2);
    play.party_icons[0].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(play.scene().sprites().len(), baseline + 2);
    Ok(())
}
